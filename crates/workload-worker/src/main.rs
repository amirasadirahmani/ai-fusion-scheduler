use afs_common::{
    monotonic_ns, process_cpu_time_ns, AdmissionDecision, TaskMetadata, TaskResult,
    WorkerLaunchSpec, WorkloadKind,
};
use afs_metadata_manager::MetadataRegistry;
use anyhow::{Context, Result};
use clap::Parser;
use std::fs::{self, File, OpenOptions};
use std::hint::black_box;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(about = "Worker process used by the AI fusion scheduler experiments")]
struct Args {
    #[arg(long)]
    spec: PathBuf,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();
    let args = Args::parse();
    let spec: WorkerLaunchSpec = serde_json::from_slice(
        &fs::read(&args.spec).with_context(|| format!("failed to read {}", args.spec.display()))?,
    )?;
    run_worker(spec)
}

fn run_worker(spec: WorkerLaunchSpec) -> Result<()> {
    let pid = std::process::id() as i32;
    let release_time_ns = spec.request_arrival_ns;
    let absolute_deadline_ns = spec.absolute_deadline_ns;
    let metadata = TaskMetadata {
        task_id: spec.task_id,
        process_id: pid,
        workflow_id: spec.workflow_id,
        stage_id: spec.stage_id,
        workload_class: spec.workload_class,
        kind: spec.kind,
        release_time_ns,
        absolute_deadline_ns,
        estimated_total_runtime_ns: spec.estimated_runtime_ns,
        estimated_remaining_runtime_ns: spec.estimated_runtime_ns,
        application_priority: spec.application_priority,
        name: spec.task_name.clone(),
    };

    let registry = MetadataRegistry::new(&spec.registry_dir);
    registry.register(&metadata)?;

    if spec.start_stopped {
        // The parent assigns SCHED_EXT or SCHED_DEADLINE while the worker is stopped.
        let rc = unsafe { libc::raise(libc::SIGSTOP) };
        if rc != 0 {
            anyhow::bail!("failed to stop worker: {}", std::io::Error::last_os_error());
        }
    }

    let start_time_ns = monotonic_ns()?;
    let cpu_start_ns = process_cpu_time_ns()?;
    let work_result = execute_work(&spec);
    let cpu_end_ns = process_cpu_time_ns().unwrap_or(cpu_start_ns);
    let completion_time_ns = monotonic_ns().unwrap_or(start_time_ns);
    let actual_cpu_time_ns = cpu_end_ns.saturating_sub(cpu_start_ns);
    let response_time_ns = completion_time_ns.saturating_sub(release_time_ns);
    let deadline_missed = absolute_deadline_ns.map(|d| completion_time_ns > d);

    let (exit_code, error) = match work_result {
        Ok(()) => (Some(0), None),
        Err(err) => (Some(1), Some(format!("{err:#}"))),
    };
    let result = TaskResult {
        experiment: spec.experiment,
        repetition: spec.repetition,
        scheduler: spec.scheduler,
        task_id: spec.task_id,
        process_id: pid,
        workflow_id: spec.workflow_id,
        stage_id: spec.stage_id,
        task_name: spec.task_name,
        workload_class: spec.workload_class,
        kind: spec.kind,
        admission: spec.admission,
        delayed_ns: spec.admission_delayed_ns,
        release_time_ns,
        start_time_ns: Some(start_time_ns),
        completion_time_ns: Some(completion_time_ns),
        absolute_deadline_ns,
        estimated_runtime_ns: spec.estimated_runtime_ns,
        actual_cpu_time_ns: Some(actual_cpu_time_ns),
        response_time_ns: Some(response_time_ns),
        deadline_missed,
        exit_code,
        error,
    };
    write_json_atomic(&spec.result_path, &result)?;
    let _ = registry.unregister(pid);

    if result.exit_code != Some(0) {
        anyhow::bail!("worker failed: {}", result.error.unwrap_or_default());
    }
    Ok(())
}

fn execute_work(spec: &WorkerLaunchSpec) -> Result<()> {
    match spec.kind {
        WorkloadKind::Cpu => cpu_burn(spec.actual_runtime_ns),
        WorkloadKind::Memory => memory_burn(spec.actual_runtime_ns, spec.memory_mb),
        WorkloadKind::Io => io_work(spec.actual_runtime_ns, spec.io_bytes, spec.task_id),
        WorkloadKind::Mixed => {
            let cpu = spec.actual_runtime_ns.saturating_mul(2) / 5;
            let mem = spec.actual_runtime_ns.saturating_mul(2) / 5;
            let io = spec.actual_runtime_ns.saturating_sub(cpu).saturating_sub(mem);
            cpu_burn(cpu)?;
            memory_burn(mem, spec.memory_mb)?;
            io_work(io, spec.io_bytes, spec.task_id)
        }
    }
}

fn cpu_burn(target_cpu_ns: u64) -> Result<()> {
    let start = process_cpu_time_ns()?;
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    loop {
        for i in 0..4096u64 {
            x ^= i.wrapping_add(x.rotate_left(13));
            x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x ^= x >> 29;
        }
        black_box(x);
        if process_cpu_time_ns()?.saturating_sub(start) >= target_cpu_ns {
            break;
        }
    }
    Ok(())
}

fn memory_burn(target_cpu_ns: u64, memory_mb: usize) -> Result<()> {
    let size = memory_mb.max(1).saturating_mul(1024 * 1024);
    let mut data = vec![0u8; size];
    let start = process_cpu_time_ns()?;
    let mut round = 1u8;
    while process_cpu_time_ns()?.saturating_sub(start) < target_cpu_ns {
        for idx in (0..data.len()).step_by(64) {
            data[idx] = data[idx].wrapping_add(round);
        }
        for idx in (0..data.len()).step_by(4096) {
            black_box(data[idx]);
        }
        round = round.wrapping_add(1);
    }
    black_box(data);
    Ok(())
}

fn io_work(target_wall_ns: u64, io_bytes: u64, task_id: u64) -> Result<()> {
    let path = std::env::temp_dir().join(format!("afs-io-{}-{task_id}.bin", std::process::id()));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)?;
    let buffer = vec![0xA5u8; 64 * 1024];
    let mut read_buffer = vec![0u8; buffer.len()];
    let start = monotonic_ns()?;
    let bytes_per_round = io_bytes.max(buffer.len() as u64);
    while monotonic_ns()?.saturating_sub(start) < target_wall_ns {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        let mut written = 0u64;
        while written < bytes_per_round {
            let n = (bytes_per_round - written).min(buffer.len() as u64) as usize;
            file.write_all(&buffer[..n])?;
            written += n as u64;
        }
        file.flush()?;
        file.seek(SeekFrom::Start(0))?;
        while file.read(&mut read_buffer)? > 0 {
            black_box(&read_buffer);
        }
    }
    drop(file);
    let _ = fs::remove_file(path);
    Ok(())
}

fn write_json_atomic(path: impl AsRef<Path>, value: &TaskResult) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let mut file = File::create(&tmp)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    Ok(())
}
