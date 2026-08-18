use afs_common::{
    apply_scheduler_policy, generate_task_manifest, monotonic_ns, proc_process_cpu_time_ns, signal,
    AdmissionDecision, ExperimentConfig, PlannedTask, SchedulerMethod, TaskMetadata, TaskResult,
    WorkerLaunchSpec,
};
use afs_metadata_manager::MetadataRegistry;
use afs_policy_core::{
    decide_admission, read_cpu_pressure, read_cpu_pressure_from, relevant_interference_ns,
    AdmissionState, CpuPressure, EwmaRuntimeEstimator, PolicyObservation, PsiLine,
};
use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Parser)]
#[command(about = "Config-driven real-process experiment runner")]
struct Args {
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long)]
    method: Option<String>,
    #[arg(long, default_value_t = 0)]
    repetition: u32,
    #[arg(long)]
    manifest: Option<PathBuf>,
    #[arg(long)]
    worker: Option<PathBuf>,
    #[arg(long)]
    no_admission: bool,
    #[arg(long)]
    no_aging: bool,
    #[arg(long)]
    no_application_deadline: bool,
    /// Diagnostic only: apply the project admission controller to a baseline.
    /// Baselines disable it by default so their native behavior remains intact.
    #[arg(long)]
    admission_on_baseline: bool,
    #[arg(long)]
    dry_run: bool,
}

struct PendingTask {
    plan: PlannedTask,
    next_eligible_ns: u64,
    request_arrival_ns: Option<u64>,
    delayed_ns: u64,
}

struct ActiveTask {
    child: Child,
    plan: PlannedTask,
    result_path: PathBuf,
    request_arrival_ns: u64,
    experiment: String,
    repetition: u32,
    scheduler: SchedulerMethod,
    admission: AdmissionDecision,
    delayed_ns: u64,
}

#[derive(Debug, Serialize)]
struct RunSummary {
    experiment: String,
    repetition: u32,
    scheduler: SchedulerMethod,
    started_ns: u64,
    finished_ns: u64,
    total_tasks: usize,
    admitted_immediately: usize,
    delayed_then_admitted: usize,
    rejected: usize,
    completed: usize,
    failed: usize,
    deadline_misses: usize,
    result_csv: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();
    let args = Args::parse();
    let mut cfg = ExperimentConfig::from_path(&args.config)?;
    if args.no_admission {
        cfg.admission.enabled = false;
    }
    if args.no_aging {
        cfg.policy.aging = false;
    }
    if args.no_application_deadline {
        cfg.policy.application_deadline = false;
    }
    let method = match args.method.as_deref() {
        Some(value) => SchedulerMethod::from_str(value).map_err(anyhow::Error::msg)?,
        None => cfg.experiment.scheduler,
    };
    if method == SchedulerMethod::Simulator {
        bail!("the real-process runner cannot use the simulator scheduler");
    }
    if method != SchedulerMethod::Proposed && !args.admission_on_baseline {
        cfg.admission.enabled = false;
    }

    fs::create_dir_all(&args.output_dir)?;
    let registry = MetadataRegistry::new(&cfg.experiment.registry_dir);
    registry.clear()?;

    let manifest_path = args
        .manifest
        .clone()
        .unwrap_or_else(|| args.output_dir.join("task-manifest.json"));
    let plans = if args.manifest.is_some() && manifest_path.exists() {
        PlannedTask::read_manifest(&manifest_path)?
    } else {
        let plans = generate_task_manifest(&cfg, args.repetition)?;
        PlannedTask::write_manifest(&plans, &manifest_path)?;
        plans
    };
    if plans.is_empty() {
        bail!("task manifest is empty");
    }
    let mut runtime_estimator =
        EwmaRuntimeEstimator::new(cfg.policy.runtime_ewma_alpha).map_err(anyhow::Error::msg)?;
    for plan in &plans {
        runtime_estimator.seed(runtime_estimation_key(plan), plan.estimated_runtime_ns);
    }

    let worker = args
        .worker
        .unwrap_or(resolve_sibling_binary("afs-workload-worker")?);
    if !worker.exists() && !args.dry_run {
        bail!(
            "worker binary not found at {}; build with scripts/build_userspace.sh or pass --worker",
            worker.display()
        );
    }

    if cfg.experiment.warmup_s > 0 && !args.dry_run {
        tracing::info!(
            seconds = cfg.experiment.warmup_s,
            "waiting for scheduler/system warm-up before releasing tasks"
        );
        thread::sleep(Duration::from_secs(cfg.experiment.warmup_s));
    }

    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_handler = Arc::clone(&stop);
    ctrlc::set_handler(move || {
        stop_for_handler.store(true, Ordering::SeqCst);
    })
    .context("failed to install Ctrl-C handler")?;

    // The harness may place this generator in a delegated systemd scope.
    // Keep workers in a child cgroup so Admission observes workload-only PSI.
    let workload_cgroup = prepare_workload_cgroup()?;

    // Complete potentially slow control-plane I/O before starting the
    // workload release clock. Otherwise filesystem latency (notably sync_all()
    // on virtualized storage) consumes application deadline budget before the
    // event loop has even started.
    let prepared_ns = monotonic_ns()?;

    write_json_atomic(
        args.output_dir.join("run-input.json"),
        &json!({
            "config": &args.config,
            "manifest": &manifest_path,
            "method": method,
            "repetition": args.repetition,
            "cpus": cfg.experiment.cpus,
            "worker": &worker,
            "policy": &cfg.policy,
            "admission": &cfg.admission,
            "workload_cgroup": workload_cgroup
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            "pressure_source": workload_cgroup
                .as_ref()
                .map(|p| p.join("cpu.pressure").to_string_lossy().into_owned())
                .unwrap_or_else(|| "/proc/pressure/cpu".to_string()),
            "prepared_ns": prepared_ns,
        }),
    )?;

    // This is the actual workload release origin.
    let started_ns = monotonic_ns()?;

    let mut pending: Vec<PendingTask> = plans
        .iter()
        .cloned()
        .map(|plan| PendingTask {
            next_eligible_ns: started_ns.saturating_add(plan.release_offset_ns),
            request_arrival_ns: None,
            delayed_ns: 0,
            plan,
        })
        .collect();

    let mut active: Vec<ActiveTask> = Vec::new();
    let mut results: Vec<TaskResult> = Vec::with_capacity(plans.len());
    let mut outcomes: HashMap<u64, bool> = HashMap::new();
    let mut completion_times: HashMap<u64, u64> = HashMap::new();

    while !pending.is_empty() || !active.is_empty() {
        if stop.load(Ordering::SeqCst) {
            terminate_children(&mut active);
            bail!("experiment interrupted; children were terminated");
        }

        reap_finished(
            &mut active,
            &mut results,
            &mut outcomes,
            &mut completion_times,
            &mut runtime_estimator,
        )?;

        let now_ns = monotonic_ns()?;
        let mut launched_any = false;
        let mut idx = 0usize;
        while idx < pending.len() && active.len() < cfg.experiment.max_concurrent {
            if now_ns < pending[idx].next_eligible_ns {
                idx += 1;
                continue;
            }

            let parent = pending[idx].plan.depends_on_task_id;
            if let Some(parent) = parent {
                match outcomes.get(&parent).copied() {
                    None => {
                        idx += 1;
                        continue;
                    }
                    Some(false) => {
                        let pending_task = pending.remove(idx);
                        let request_arrival_ns = pending_task
                            .request_arrival_ns
                            .unwrap_or(pending_task.next_eligible_ns);
                        let absolute_deadline_ns = pending_task
                            .plan
                            .relative_deadline_ns
                            .map(|d| request_arrival_ns.saturating_add(d));
                        let mut result = TaskResult::rejected(
                            cfg.experiment.name.clone(),
                            args.repetition,
                            method,
                            pending_task.plan.task_id,
                            pending_task.plan.workflow_id,
                            pending_task.plan.stage_id,
                            pending_task.plan.name,
                            pending_task.plan.workload_class,
                            pending_task.plan.kind,
                            request_arrival_ns,
                            absolute_deadline_ns,
                            pending_task.plan.estimated_runtime_ns,
                            pending_task.delayed_ns,
                        );
                        result.error =
                            Some(format!("dependency task {parent} failed or was rejected"));
                        outcomes.insert(result.task_id, false);
                        results.push(result);
                        launched_any = true;
                        continue;
                    }
                    Some(true) => {}
                }
            }

            let mut pending_task = pending.remove(idx);
            let planned_release_ns = started_ns.saturating_add(pending_task.plan.release_offset_ns);
            let dependency_ready_ns = pending_task
                .plan
                .depends_on_task_id
                .and_then(|id| completion_times.get(&id).copied())
                .unwrap_or(planned_release_ns);
            let request_arrival_ns = *pending_task
                .request_arrival_ns
                .get_or_insert(planned_release_ns.max(dependency_ready_ns));

            let runtime_key = runtime_estimation_key(&pending_task.plan);
            pending_task.plan.estimated_runtime_ns =
                runtime_estimator.estimate(&runtime_key, pending_task.plan.estimated_runtime_ns);

            if args.dry_run {
                println!(
                    "DRY-RUN launch task={} name={} release={} deadline={:?}",
                    pending_task.plan.task_id,
                    pending_task.plan.name,
                    request_arrival_ns,
                    pending_task.plan.relative_deadline_ns
                );
                outcomes.insert(pending_task.plan.task_id, true);
                completion_times.insert(
                    pending_task.plan.task_id,
                    request_arrival_ns.saturating_add(pending_task.plan.actual_runtime_ns),
                );
                let mut result = dry_run_result(
                    &cfg,
                    &pending_task.plan,
                    args.repetition,
                    method,
                    request_arrival_ns,
                );
                if pending_task.delayed_ns > 0 {
                    result.admission = AdmissionDecision::Delay;
                    result.delayed_ns = pending_task.delayed_ns;
                }
                results.push(result);
                if pending_task.plan.kind != afs_common::WorkloadKind::Io {
                    runtime_estimator.observe(&runtime_key, pending_task.plan.actual_runtime_ns);
                }
                launched_any = true;
                continue;
            }

            match launch_task(
                &cfg,
                &args.output_dir,
                &worker,
                &registry,
                method,
                args.repetition,
                pending_task.plan,
                request_arrival_ns,
                pending_task.delayed_ns,
                &active,
                workload_cgroup.as_deref(),
            )? {
                LaunchOutcome::Spawned(task) => active.push(task),
                LaunchOutcome::Delayed {
                    plan,
                    request_arrival_ns,
                    delayed_ns,
                    next_eligible_ns,
                } => pending.push(PendingTask {
                    plan,
                    next_eligible_ns,
                    request_arrival_ns: Some(request_arrival_ns),
                    delayed_ns,
                }),
                LaunchOutcome::Rejected(result) => {
                    outcomes.insert(result.task_id, false);
                    results.push(result);
                }
            }
            launched_any = true;
        }

        if !launched_any {
            if active.is_empty() {
                let next_release = pending
                    .iter()
                    .filter(|p| {
                        p.plan
                            .depends_on_task_id
                            .map(|parent| outcomes.contains_key(&parent))
                            .unwrap_or(true)
                    })
                    .map(|p| p.next_eligible_ns)
                    .min();
                match next_release {
                    Some(next_release) => sleep_until(next_release)?,
                    None if !pending.is_empty() => {
                        bail!("workload dependency deadlock or cyclic manifest")
                    }
                    None => {}
                }
            } else {
                thread::sleep(Duration::from_millis(1));
            }
        }
    }

    results.sort_by_key(|r| r.task_id);
    let result_csv = args.output_dir.join("tasks.csv");
    write_csv(&result_csv, &results)?;
    let finished_ns = monotonic_ns()?;
    let summary = summarize(
        &cfg,
        method,
        args.repetition,
        started_ns,
        finished_ns,
        &result_csv,
        &results,
    );
    if let Some(cgroup) = workload_cgroup.as_ref() {
        let pressure_path = cgroup.join("cpu.pressure");
        if let Ok(text) = fs::read_to_string(&pressure_path) {
            fs::write(
                args.output_dir.join("workload-cpu-pressure-final.txt"),
                text,
            )?;
        }
    }

    write_json_atomic(args.output_dir.join("summary.json"), &summary)?;
    write_json_atomic(
        args.output_dir.join("runtime-estimates.json"),
        &runtime_estimator.snapshot(),
    )?;
    println!(
        "experiment={} method={} tasks={} completed={} rejected={} failed={} misses={} output={}",
        summary.experiment,
        summary.scheduler,
        summary.total_tasks,
        summary.completed,
        summary.rejected,
        summary.failed,
        summary.deadline_misses,
        result_csv.display()
    );
    Ok(())
}

enum LaunchOutcome {
    Spawned(ActiveTask),
    Delayed {
        plan: PlannedTask,
        request_arrival_ns: u64,
        delayed_ns: u64,
        next_eligible_ns: u64,
    },
    Rejected(TaskResult),
}

fn prepare_workload_cgroup() -> Result<Option<PathBuf>> {
    if std::env::var_os("AFS_WORKLOAD_CGROUP_SCOPED").is_none() {
        return Ok(None);
    }

    let membership =
        fs::read_to_string("/proc/self/cgroup").context("failed to read /proc/self/cgroup")?;

    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .context("cgroup v2 membership not found in /proc/self/cgroup")?;

    let base = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));

    let workload = base.join("afs-workload");

    if workload.exists() {
        fs::remove_dir(&workload).with_context(|| {
            format!(
                "failed to remove stale workload cgroup {}",
                workload.display()
            )
        })?;
    }

    fs::create_dir(&workload).with_context(|| {
        format!(
            "failed to create delegated workload cgroup {}",
            workload.display()
        )
    })?;

    let pressure = workload.join("cpu.pressure");
    if !pressure.is_file() {
        anyhow::bail!(
            "workload cgroup has no cpu.pressure file: {}",
            pressure.display()
        );
    }

    tracing::info!(
        cgroup = %workload.display(),
        pressure = %pressure.display(),
        "using workload-scoped CPU PSI"
    );

    Ok(Some(workload))
}

fn move_pid_to_workload_cgroup(cgroup: Option<&Path>, pid: i32) -> Result<()> {
    let Some(cgroup) = cgroup else {
        return Ok(());
    };

    let procs = cgroup.join("cgroup.procs");

    fs::write(&procs, format!("{pid}\n")).with_context(|| {
        format!(
            "failed to move pid {pid} into workload cgroup {}",
            cgroup.display()
        )
    })?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn launch_task(
    cfg: &ExperimentConfig,
    output_dir: &Path,
    worker: &Path,
    registry: &MetadataRegistry,
    method: SchedulerMethod,
    repetition: u32,
    plan: PlannedTask,
    request_arrival_ns: u64,
    delayed_ns: u64,
    active: &[ActiveTask],
    workload_cgroup: Option<&Path>,
) -> Result<LaunchOutcome> {
    let absolute_deadline_ns = plan
        .relative_deadline_ns
        .map(|d| request_arrival_ns.saturating_add(d));
    let now_ns = monotonic_ns()?;
    let policy_deadline_ns = if cfg.policy.application_deadline {
        absolute_deadline_ns
    } else {
        None
    };
    let metadata = TaskMetadata {
        task_id: plan.task_id,
        process_id: 0,
        workflow_id: plan.workflow_id,
        stage_id: plan.stage_id,
        workload_class: plan.workload_class,
        kind: plan.kind,
        release_time_ns: request_arrival_ns,
        absolute_deadline_ns: policy_deadline_ns,
        estimated_total_runtime_ns: plan.estimated_runtime_ns,
        estimated_remaining_runtime_ns: plan.estimated_runtime_ns,
        application_priority: plan.application_priority,
        name: plan.name.clone(),
    };
    let pressure = if let Some(cgroup) = workload_cgroup {
        let path = cgroup.join("cpu.pressure");
        read_cpu_pressure_from(&path).with_context(|| {
            format!(
                "failed to read workload-scoped CPU PSI from {}",
                path.display()
            )
        })?
    } else {
        read_cpu_pressure().unwrap_or(CpuPressure {
            some: PsiLine::default(),
            full: None,
        })
    };
    let active_remaining =
        estimate_relevant_active_remaining(active, &metadata, now_ns, &cfg.policy);
    let decision = decide_admission(
        &metadata,
        now_ns,
        AdmissionState {
            active_estimated_remaining_ns: active_remaining,
            active_tasks: active.len(),
            cpus: cfg.experiment.cpus,
            pressure,
        },
        &cfg.admission,
    );

    if std::env::var_os("AFS_TRACE_ADMISSION").is_some() {
        let decision_lag_ms = now_ns.saturating_sub(request_arrival_ns) as f64 / 1_000_000.0;

        let deadline_left_ms =
            policy_deadline_ns.map(|deadline| deadline.saturating_sub(now_ns) as f64 / 1_000_000.0);

        let estimated_ms = metadata.estimated_remaining_runtime_ns as f64 / 1_000_000.0;

        let relevant_ms = active_remaining as f64 / 1_000_000.0;

        let predicted_ms = decision.predicted_finish_ns.saturating_sub(now_ns) as f64 / 1_000_000.0;

        let deadline_left_text = deadline_left_ms
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "-".to_string());

        eprintln!(
            "AFS_ADMISSION_TRACE \
task={} class={:?} delayed_ms={:.3} \
decision_lag_ms={:.3} deadline_left_ms={} \
estimated_ms={:.3} relevant_ms={:.3} \
active={} psi_avg10={:.2} predicted_ms={:.3} \
decision={:?} reason={}",
            metadata.task_id,
            metadata.workload_class,
            delayed_ns as f64 / 1_000_000.0,
            decision_lag_ms,
            deadline_left_text,
            estimated_ms,
            relevant_ms,
            active.len(),
            pressure.some.avg10,
            predicted_ms,
            decision.decision,
            decision.reason,
        );
    }

    let final_admission = match decision.decision {
        AdmissionDecision::Admit => {
            if delayed_ns == 0 {
                AdmissionDecision::Admit
            } else {
                AdmissionDecision::Delay
            }
        }
        AdmissionDecision::Reject => {
            let mut result = TaskResult::rejected(
                cfg.experiment.name.clone(),
                repetition,
                method,
                plan.task_id,
                plan.workflow_id,
                plan.stage_id,
                plan.name,
                plan.workload_class,
                plan.kind,
                request_arrival_ns,
                absolute_deadline_ns,
                plan.estimated_runtime_ns,
                delayed_ns,
            );
            result.error = Some(decision.reason.into());
            return Ok(LaunchOutcome::Rejected(result));
        }
        AdmissionDecision::Delay => {
            let step_ns = cfg.admission.delay_step_ms.saturating_mul(1_000_000);
            let max_ns = cfg.admission.max_delay_ms.saturating_mul(1_000_000);
            if step_ns == 0 || delayed_ns.saturating_add(step_ns) > max_ns {
                let mut result = TaskResult::rejected(
                    cfg.experiment.name.clone(),
                    repetition,
                    method,
                    plan.task_id,
                    plan.workflow_id,
                    plan.stage_id,
                    plan.name,
                    plan.workload_class,
                    plan.kind,
                    request_arrival_ns,
                    absolute_deadline_ns,
                    plan.estimated_runtime_ns,
                    delayed_ns,
                );
                result.error = Some(format!(
                    "maximum admission delay exceeded; last reason: {}",
                    decision.reason
                ));
                return Ok(LaunchOutcome::Rejected(result));
            }
            return Ok(LaunchOutcome::Delayed {
                plan,
                request_arrival_ns,
                delayed_ns: delayed_ns.saturating_add(step_ns),
                next_eligible_ns: now_ns.saturating_add(step_ns),
            });
        }
    };

    let task_dir = output_dir.join("tasks");
    fs::create_dir_all(&task_dir)?;
    let spec_path = task_dir.join(format!("task-{:06}.spec.json", plan.task_id));
    let result_path = task_dir.join(format!("task-{:06}.result.json", plan.task_id));
    let spec = WorkerLaunchSpec {
        experiment: cfg.experiment.name.clone(),
        repetition,
        scheduler: method,
        admission: final_admission,
        task_id: plan.task_id,
        task_name: plan.name.clone(),
        workflow_id: plan.workflow_id,
        stage_id: plan.stage_id,
        workload_class: plan.workload_class,
        kind: plan.kind,
        estimated_runtime_ns: plan.estimated_runtime_ns,
        actual_runtime_ns: plan.actual_runtime_ns,
        request_arrival_ns,
        absolute_deadline_ns,
        application_priority: plan.application_priority,
        memory_mb: plan.memory_mb,
        io_bytes: plan.io_bytes,
        sched_period_ns: plan.sched_period_ns,
        registry_dir: cfg.experiment.registry_dir.clone(),
        result_path: result_path.to_string_lossy().into_owned(),
        admission_delayed_ns: delayed_ns,
        start_stopped: true,
    };
    write_json_atomic(&spec_path, &spec)?;

    let mut command = if let Ok(cpu_list) = std::env::var("AFS_WORKLOAD_CPU_LIST") {
        let mut cmd = Command::new("taskset");
        cmd.arg("-c").arg(cpu_list).arg(worker);
        cmd
    } else {
        Command::new(worker)
    };

    let mut child = command
        .arg("--spec")
        .arg(&spec_path)
        .spawn()
        .with_context(|| format!("failed to start worker {}", worker.display()))?;
    let pid = child.id() as i32;
    if let Err(err) = wait_for_worker_ready(registry, pid, Duration::from_secs(5)) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(err).context(format!("worker {pid} failed before becoming ready"));
    }

    // The worker starts stopped. Move it into the workload-only cgroup
    // before applying SCHED_EXT/SCHED_DEADLINE and before SIGCONT so its
    // runtime pressure is accounted only to the workload cgroup.
    if let Err(err) = move_pid_to_workload_cgroup(workload_cgroup, pid) {
        let _ = signal(pid, libc::SIGCONT);
        let _ = child.kill();
        let _ = child.wait();
        let _ = registry.unregister(pid);
        return Err(err);
    }

    let policy_result = match method {
        SchedulerMethod::SchedDeadline => {
            let relative_deadline = plan
                .relative_deadline_ns
                .context("SCHED_DEADLINE requires deadline_ms")?;
            let period = plan.sched_period_ns.unwrap_or(relative_deadline);
            // A small headroom avoids invalid runtime > deadline after estimation jitter.
            let runtime = plan
                .estimated_runtime_ns
                .saturating_mul(11)
                .saturating_div(10)
                .min(relative_deadline);
            apply_scheduler_policy(method, pid, runtime, Some(relative_deadline), Some(period))
        }
        _ => apply_scheduler_policy(method, pid, plan.estimated_runtime_ns, None, None),
    };
    if let Err(err) = policy_result {
        let _ = signal(pid, libc::SIGCONT);
        let _ = child.kill();
        let _ = child.wait();
        let _ = registry.unregister(pid);

        // SCHED_DEADLINE performs kernel-side bandwidth admission. EBUSY is an
        // experimental outcome, not a harness failure; report it as a rejected
        // task so the run can continue. Other errors indicate environment or
        // parameter problems and remain fatal.
        if method == SchedulerMethod::SchedDeadline && err.raw_os_error() == Some(libc::EBUSY) {
            let mut result = TaskResult::rejected(
                cfg.experiment.name.clone(),
                repetition,
                method,
                plan.task_id,
                plan.workflow_id,
                plan.stage_id,
                plan.name,
                plan.workload_class,
                plan.kind,
                request_arrival_ns,
                absolute_deadline_ns,
                plan.estimated_runtime_ns,
                delayed_ns,
            );
            result.error = Some("kernel_sched_deadline_bandwidth_rejected".into());
            return Ok(LaunchOutcome::Rejected(result));
        }
        return Err(err).context(format!("failed to apply {method} to pid {pid}"));
    }
    signal(pid, libc::SIGCONT).with_context(|| format!("failed to continue pid {pid}"))?;

    Ok(LaunchOutcome::Spawned(ActiveTask {
        child,
        plan,
        result_path,
        request_arrival_ns,
        experiment: cfg.experiment.name.clone(),
        repetition,
        scheduler: method,
        admission: final_admission,
        delayed_ns,
    }))
}

fn estimate_active_remaining(task: &ActiveTask) -> u64 {
    let consumed = proc_process_cpu_time_ns(task.child.id() as i32).unwrap_or(0);
    task.plan.estimated_runtime_ns.saturating_sub(consumed)
}

fn estimate_relevant_active_remaining(
    active: &[ActiveTask],
    incoming: &TaskMetadata,
    now_ns: u64,
    policy: &afs_common::PolicyConfig,
) -> u64 {
    let incoming_obs = PolicyObservation {
        metadata: incoming.clone(),
        now_ns,
        consumed_cpu_ns: 0,
        last_enqueue_ns: now_ns,
        virtual_deadline_ns: None,
    };

    let observations: Vec<(PolicyObservation, u64)> = active
        .iter()
        .map(|task| {
            let remaining_ns = estimate_active_remaining(task);
            let consumed_ns = task.plan.estimated_runtime_ns.saturating_sub(remaining_ns);

            let absolute_deadline_ns = if policy.application_deadline {
                task.plan
                    .relative_deadline_ns
                    .map(|deadline| task.request_arrival_ns.saturating_add(deadline))
            } else {
                None
            };

            let metadata = TaskMetadata {
                task_id: task.plan.task_id,
                process_id: task.child.id() as i32,
                workflow_id: task.plan.workflow_id,
                stage_id: task.plan.stage_id,
                workload_class: task.plan.workload_class,
                kind: task.plan.kind,
                release_time_ns: task.request_arrival_ns,
                absolute_deadline_ns,
                estimated_total_runtime_ns: task.plan.estimated_runtime_ns,
                estimated_remaining_runtime_ns: remaining_ns,
                application_priority: task.plan.application_priority,
                name: task.plan.name.clone(),
            };

            let obs = PolicyObservation {
                metadata,
                now_ns,
                consumed_cpu_ns: consumed_ns,
                // Userspace does not observe every kernel re-enqueue. Using the
                // request time is a conservative aging proxy for admission.
                last_enqueue_ns: task.request_arrival_ns,
                virtual_deadline_ns: None,
            };

            (obs, remaining_ns)
        })
        .collect();

    relevant_interference_ns(&incoming_obs, &observations, policy)
}

fn reap_finished(
    active: &mut Vec<ActiveTask>,
    results: &mut Vec<TaskResult>,
    outcomes: &mut HashMap<u64, bool>,
    completion_times: &mut HashMap<u64, u64>,
    runtime_estimator: &mut EwmaRuntimeEstimator,
) -> Result<()> {
    let mut idx = 0usize;
    while idx < active.len() {
        let status = active[idx].child.try_wait()?;
        if let Some(status) = status {
            let finished = active.swap_remove(idx);
            let result = read_worker_result(&finished, status)?;
            let success = result.exit_code == Some(0) && result.error.is_none();
            outcomes.insert(result.task_id, success);
            if let Some(completion) = result.completion_time_ns {
                completion_times.insert(result.task_id, completion);
            }
            if success && finished.plan.kind != afs_common::WorkloadKind::Io {
                if let Some(actual_cpu_ns) = result.actual_cpu_time_ns {
                    runtime_estimator
                        .observe(&runtime_estimation_key(&finished.plan), actual_cpu_ns);
                }
            }
            results.push(result);
        } else {
            idx += 1;
        }
    }
    Ok(())
}

fn read_worker_result(task: &ActiveTask, status: ExitStatus) -> Result<TaskResult> {
    match fs::read(&task.result_path) {
        Ok(data) => serde_json::from_slice(&data)
            .with_context(|| format!("invalid worker result {}", task.result_path.display())),
        Err(err) => Ok(TaskResult {
            experiment: task.experiment.clone(),
            repetition: task.repetition,
            scheduler: task.scheduler,
            task_id: task.plan.task_id,
            process_id: task.child.id() as i32,
            workflow_id: task.plan.workflow_id,
            stage_id: task.plan.stage_id,
            task_name: task.plan.name.clone(),
            workload_class: task.plan.workload_class,
            kind: task.plan.kind,
            admission: task.admission,
            delayed_ns: task.delayed_ns,
            release_time_ns: task.request_arrival_ns,
            start_time_ns: None,
            completion_time_ns: monotonic_ns().ok(),
            absolute_deadline_ns: task
                .plan
                .relative_deadline_ns
                .map(|d| task.request_arrival_ns.saturating_add(d)),
            estimated_runtime_ns: task.plan.estimated_runtime_ns,
            actual_cpu_time_ns: None,
            response_time_ns: None,
            deadline_missed: None,
            exit_code: status.code(),
            error: Some(format!(
                "worker exited without result file (status={status}, read_error={err})"
            )),
        }),
    }
}

fn runtime_estimation_key(plan: &PlannedTask) -> String {
    if plan.workflow_id != 0 && plan.stage_id != 0 {
        format!("pipeline-stage-{}-{}", plan.stage_id, plan.kind)
    } else {
        let base = plan
            .name
            .rsplit_once('-')
            .and_then(|(prefix, suffix)| suffix.parse::<u64>().ok().map(|_| prefix))
            .unwrap_or(plan.name.as_str());
        format!("{}-{}", base, plan.kind)
    }
}

fn wait_for_worker_ready(registry: &MetadataRegistry, pid: i32, timeout: Duration) -> Result<()> {
    let started = Instant::now();
    loop {
        let registered = registry.load(pid)?.is_some();
        if registered && process_is_stopped(pid).unwrap_or(false) {
            return Ok(());
        }
        if started.elapsed() >= timeout {
            bail!("timed out waiting for worker metadata registration and SIGSTOP");
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn process_is_stopped(pid: i32) -> std::io::Result<bool> {
    let status = fs::read_to_string(format!("/proc/{pid}/status"))?;
    Ok(status.lines().any(|line| {
        line.strip_prefix("State:")
            .map(|state| {
                let state = state.trim_start();
                state.starts_with('T') || state.starts_with('t')
            })
            .unwrap_or(false)
    }))
}

fn sleep_until(target_ns: u64) -> Result<()> {
    loop {
        let now = monotonic_ns()?;
        if now >= target_ns {
            return Ok(());
        }
        let remaining = target_ns - now;
        thread::sleep(Duration::from_nanos(remaining.min(10_000_000)));
    }
}

fn resolve_sibling_binary(name: &str) -> Result<PathBuf> {
    let current = std::env::current_exe()?;
    let parent = current
        .parent()
        .context("current executable has no parent")?;
    Ok(parent.join(name))
}

fn terminate_children(active: &mut [ActiveTask]) {
    for task in active.iter_mut() {
        let _ = task.child.kill();
        let _ = task.child.wait();
    }
}

fn write_csv(path: &Path, results: &[TaskResult]) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;
    for result in results {
        writer.serialize(result)?;
    }
    writer.flush()?;
    Ok(())
}

fn write_json_atomic(path: impl AsRef<Path>, value: &impl Serialize) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let mut file = File::create(&tmp)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn dry_run_result(
    cfg: &ExperimentConfig,
    plan: &PlannedTask,
    repetition: u32,
    method: SchedulerMethod,
    release_ns: u64,
) -> TaskResult {
    let completion = release_ns.saturating_add(plan.actual_runtime_ns);
    let deadline = plan
        .relative_deadline_ns
        .map(|d| release_ns.saturating_add(d));
    TaskResult {
        experiment: cfg.experiment.name.clone(),
        repetition,
        scheduler: method,
        task_id: plan.task_id,
        process_id: plan.task_id as i32,
        workflow_id: plan.workflow_id,
        stage_id: plan.stage_id,
        task_name: plan.name.clone(),
        workload_class: plan.workload_class,
        kind: plan.kind,
        admission: AdmissionDecision::Admit,
        delayed_ns: 0,
        release_time_ns: release_ns,
        start_time_ns: Some(release_ns),
        completion_time_ns: Some(completion),
        absolute_deadline_ns: deadline,
        estimated_runtime_ns: plan.estimated_runtime_ns,
        actual_cpu_time_ns: Some(plan.actual_runtime_ns),
        response_time_ns: Some(plan.actual_runtime_ns),
        deadline_missed: deadline.map(|d| completion > d),
        exit_code: Some(0),
        error: None,
    }
}

fn summarize(
    cfg: &ExperimentConfig,
    scheduler: SchedulerMethod,
    repetition: u32,
    started_ns: u64,
    finished_ns: u64,
    result_csv: &Path,
    results: &[TaskResult],
) -> RunSummary {
    RunSummary {
        experiment: cfg.experiment.name.clone(),
        repetition,
        scheduler,
        started_ns,
        finished_ns,
        total_tasks: results.len(),
        admitted_immediately: results
            .iter()
            .filter(|r| r.admission == AdmissionDecision::Admit)
            .count(),
        delayed_then_admitted: results
            .iter()
            .filter(|r| r.admission == AdmissionDecision::Delay)
            .count(),
        rejected: results
            .iter()
            .filter(|r| r.admission == AdmissionDecision::Reject)
            .count(),
        completed: results
            .iter()
            .filter(|r| r.completion_time_ns.is_some() && r.exit_code == Some(0))
            .count(),
        failed: results
            .iter()
            .filter(|r| r.exit_code.is_some() && r.exit_code != Some(0))
            .count(),
        deadline_misses: results
            .iter()
            .filter(|r| r.deadline_missed == Some(true))
            .count(),
        result_csv: result_csv.to_string_lossy().into_owned(),
    }
}
