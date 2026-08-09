// SPDX-License-Identifier: GPL-2.0-only
//
// Target-host adapter for scx_rustland_core 2.4.13. The policy itself lives in
// afs-policy-core and is unit-testable without a sched_ext kernel.

mod bpf_skel;
pub use bpf_skel::*;
pub mod bpf_intf;
#[rustfmt::skip]
mod bpf;
use bpf::*;

use afs_common::{
    monotonic_ns, proc_process_cpu_time_ns, process_cpu_time_ns, ExperimentConfig,
    SchedulerSnapshot, TaskMetadata, WorkloadClass, WorkloadKind,
};
use afs_metadata_manager::MetadataRegistry;
use afs_policy_core::{class_slice_ns, score_task, PolicyObservation};
use anyhow::{bail, Context, Result};
use clap::Parser;
use libbpf_rs::OpenObject;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Parser)]
#[command(about = "Application-aware soft-deadline sched_ext policy in Rust user space")]
struct Args {
    /// Experiment configuration. Only [policy] and registry_dir are consumed.
    #[arg(long, default_value = "configs/smoke.toml")]
    config: PathBuf,

    /// Override the PID-keyed metadata registry directory.
    #[arg(long)]
    registry_dir: Option<PathBuf>,

    /// Periodically write scheduler counters to this JSON file.
    #[arg(long, default_value = "run/proposed-scheduler-stats.json")]
    stats_json: PathBuf,

    /// Maximum number of queued tasks consumed before one dispatch cycle.
    #[arg(long, default_value_t = 256)]
    max_batch: usize,

    /// Snapshot interval for the JSON counters.
    #[arg(long, default_value_t = 500)]
    stats_interval_ms: u64,

    /// Exit-info buffer length passed to rustland_core (0 = library default).
    #[arg(long, default_value_t = 0)]
    exit_dump_len: u32,

    /// Manage only processes explicitly moved to SCHED_EXT. Recommended.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    partial: bool,

    /// Enable verbose rustland_core/BPF diagnostics.
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Disable the application deadline and use a rustland-like virtual deadline.
    #[arg(long)]
    no_application_deadline: bool,

    /// Disable explicit aging (ablation).
    #[arg(long)]
    no_aging: bool,

    /// Disable application priority (diagnostic ablation).
    #[arg(long)]
    no_priority: bool,
}

#[derive(Debug)]
struct RankedTask {
    queued: QueuedTask,
    metadata: TaskMetadata,
    score_ns: i128,
    slice_ns: u64,
}

#[derive(Debug, Default, Serialize)]
struct AdapterCounters {
    started_ns: u64,
    last_snapshot_ns: u64,
    metadata_cache_hits: u64,
    metadata_registry_reads: u64,
    metadata_misses: u64,
    dequeue_errors: u64,
    dispatch_errors: u64,
    local_dispatches: u64,
    dispatch_attempts: u64,
    tasks_dequeued: u64,
    tasks_in_notify_cycles: u64,
    notify_cycles: u64,
    max_batch_seen: usize,
    proc_runtime_reads: u64,
    proc_runtime_read_failures: u64,
    scheduler_cpu_time_ns: u64,
    total_decision_wall_ns: u64,
    max_decision_wall_ns: u64,
    total_cycle_wall_ns: u64,
    max_cycle_wall_ns: u64,
    core: SchedulerSnapshot,
}

struct Scheduler<'a> {
    bpf: BpfScheduler<'a>,
    registry: MetadataRegistry,
    cfg: ExperimentConfig,
    queue: Vec<RankedTask>,
    metadata_cache: HashMap<i32, TaskMetadata>,
    counters: AdapterCounters,
    stats_json: PathBuf,
    max_batch: usize,
    stats_interval: Duration,
    last_stats: Instant,
    scheduler_cpu_start_ns: u64,
}

impl<'a> Scheduler<'a> {
    fn init(
        open_object: &'a mut MaybeUninit<OpenObject>,
        args: &Args,
        mut cfg: ExperimentConfig,
    ) -> Result<Self> {
        if args.max_batch == 0 {
            bail!("--max-batch must be positive");
        }
        cfg.policy.application_deadline &= !args.no_application_deadline;
        cfg.policy.aging &= !args.no_aging;
        cfg.policy.priority &= !args.no_priority;

        #[cfg(feature = "rustland-init5")]
        let bpf = BpfScheduler::init(
            open_object,
            None, // default libbpf open options
            args.exit_dump_len,
            args.partial,
            args.debug,
            false,     // builtin_idle: keep CPU selection in rustland_core/user policy
            false,     // numa_local
            1_000_000, // 1000 us, matching upstream rustland slice_us_min
            "afs_rustland",
        )?;

        #[cfg(all(not(feature = "rustland-init5"), feature = "rustland-init4"))]
        let bpf = BpfScheduler::init(open_object, args.exit_dump_len, args.partial, args.debug)?;

        #[cfg(all(not(feature = "rustland-init5"), not(feature = "rustland-init4")))]
        compile_error!("enable exactly one rustland init compatibility feature");

        let registry_dir = args
            .registry_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(&cfg.experiment.registry_dir));
        let registry = MetadataRegistry::new(registry_dir);
        registry.initialize()?;

        let started_ns = monotonic_ns()?;
        let scheduler_cpu_start_ns = process_cpu_time_ns().unwrap_or_default();
        Ok(Self {
            bpf,
            registry,
            cfg,
            queue: Vec::with_capacity(args.max_batch),
            metadata_cache: HashMap::new(),
            counters: AdapterCounters {
                started_ns,
                last_snapshot_ns: started_ns,
                ..AdapterCounters::default()
            },
            stats_json: args.stats_json.clone(),
            max_batch: args.max_batch,
            stats_interval: Duration::from_millis(args.stats_interval_ms.max(1)),
            last_stats: Instant::now(),
            scheduler_cpu_start_ns,
        })
    }

    fn fallback_metadata(&self, task: &QueuedTask, now_ns: u64) -> TaskMetadata {
        // Unknown tasks should only occur if partial mode was disabled or if a worker
        // exited between registration and dequeue. Preserve progress with a relaxed,
        // non-critical policy rather than blocking the BPF queue.
        let estimated = task
            .exec_runtime
            .max(self.cfg.policy.quantum_ms * 1_000_000);
        TaskMetadata {
            task_id: task.pid.max(0) as u64,
            process_id: task.pid,
            workflow_id: 0,
            stage_id: 0,
            workload_class: WorkloadClass::Interactive,
            kind: WorkloadKind::Cpu,
            release_time_ns: task.stop_ts.min(now_ns),
            absolute_deadline_ns: None,
            estimated_total_runtime_ns: estimated,
            estimated_remaining_runtime_ns: estimated,
            application_priority: (task.weight as f64 / 10_000.0).clamp(0.0, 1.0),
            name: format!("unregistered-pid-{}", task.pid),
        }
    }

    fn consumed_cpu_ns(&mut self, task: &QueuedTask) -> u64 {
        if self.cfg.policy.runtime_source.eq_ignore_ascii_case("proc") {
            self.counters.proc_runtime_reads += 1;
            match proc_process_cpu_time_ns(task.pid) {
                Ok(value) => value,
                Err(err) => {
                    self.counters.proc_runtime_read_failures += 1;
                    tracing::debug!(pid = task.pid, error = %err, "falling back to queued exec_runtime");
                    task.exec_runtime
                }
            }
        } else {
            task.exec_runtime
        }
    }

    fn consume_available(&mut self) {
        while self.queue.len() < self.max_batch {
            match self.bpf.dequeue_task() {
                Ok(Some(task)) => {
                    self.counters.tasks_dequeued += 1;
                    // Negative CPU is rustland_core's task-exit notification.
                    if task.cpu < 0 {
                        self.metadata_cache.remove(&task.pid);
                        let _ = self.registry.unregister(task.pid);
                        continue;
                    }
                    let now_ns = monotonic_ns().unwrap_or(task.stop_ts);
                    let metadata = if let Some(metadata) = self.metadata_cache.get(&task.pid) {
                        self.counters.metadata_cache_hits += 1;
                        metadata.clone()
                    } else {
                        self.counters.metadata_registry_reads += 1;
                        match self.registry.load(task.pid) {
                            Ok(Some(metadata)) => {
                                self.metadata_cache.insert(task.pid, metadata.clone());
                                metadata
                            }
                            Ok(None) => {
                                self.counters.metadata_misses += 1;
                                self.fallback_metadata(&task, now_ns)
                            }
                            Err(err) => {
                                self.counters.metadata_misses += 1;
                                tracing::warn!(pid = task.pid, error = %err, "metadata read failed");
                                self.fallback_metadata(&task, now_ns)
                            }
                        }
                    };
                    let consumed_cpu_ns = self.consumed_cpu_ns(&task);
                    let virtual_deadline_ns =
                        Some(task.vtime.saturating_add(task.exec_runtime.max(1)));
                    let observation = PolicyObservation {
                        metadata: metadata.clone(),
                        now_ns,
                        consumed_cpu_ns,
                        last_enqueue_ns: task.stop_ts.min(now_ns),
                        virtual_deadline_ns,
                    };
                    let breakdown = score_task(&observation, &self.cfg.policy);
                    self.queue.push(RankedTask {
                        queued: task,
                        metadata,
                        score_ns: breakdown.score_ns,
                        slice_ns: class_slice_ns(
                            observation.metadata.workload_class,
                            &self.cfg.policy,
                        ),
                    });
                }
                Ok(None) => break,
                Err(err) => {
                    self.counters.dequeue_errors += 1;
                    tracing::warn!(error = %err, "dequeue_task failed");
                    break;
                }
            }
        }
        self.counters.max_batch_seen = self.counters.max_batch_seen.max(self.queue.len());
    }

    fn dispatch_batch(&mut self) {
        let decision_started_ns = monotonic_ns().unwrap_or_default();
        self.queue.sort_by(|a, b| {
            a.score_ns
                .cmp(&b.score_ns)
                .then_with(|| a.metadata.release_time_ns.cmp(&b.metadata.release_time_ns))
                .then_with(|| a.metadata.task_id.cmp(&b.metadata.task_id))
        });
        let min_score = self.queue.first().map(|t| t.score_ns).unwrap_or(0);
        let batch = std::mem::take(&mut self.queue);
        let batch_len = batch.len();
        self.counters.tasks_in_notify_cycles = self
            .counters
            .tasks_in_notify_cycles
            .saturating_add(batch_len as u64);
        let mut pending_after_error = Vec::new();
        let mut iter = batch.into_iter();

        while let Some(ranked) = iter.next() {
            let mut decision = DispatchedTask::new(&ranked.queued);
            decision.slice_ns = ranked.slice_ns;
            decision.vtime = relative_vtime(ranked.score_ns, min_score);

            // Reuse rustland_core's topology-aware idle-CPU helper. If no idle
            // CPU is found, retain the previous CPU and let the BPF backend
            // account for any affinity bounce.
            let selected =
                self.bpf
                    .select_cpu(ranked.queued.pid, ranked.queued.cpu, ranked.queued.flags);
            decision.cpu = if selected >= 0 {
                selected
            } else {
                ranked.queued.cpu
            };

            self.counters.dispatch_attempts += 1;
            match self.bpf.dispatch_task(&decision) {
                Ok(()) => {
                    self.counters.local_dispatches += 1;
                }
                Err(err) => {
                    self.counters.dispatch_errors += 1;
                    tracing::warn!(pid = ranked.queued.pid, error = %err, "dispatch failed");
                    pending_after_error.push(ranked);
                    pending_after_error.extend(iter);
                    break;
                }
            }
        }

        self.queue = pending_after_error;
        let decision_finished_ns = monotonic_ns().unwrap_or(decision_started_ns);
        let decision_wall_ns = decision_finished_ns.saturating_sub(decision_started_ns);
        self.counters.total_decision_wall_ns = self
            .counters
            .total_decision_wall_ns
            .saturating_add(decision_wall_ns);
        self.counters.max_decision_wall_ns =
            self.counters.max_decision_wall_ns.max(decision_wall_ns);
        self.counters.notify_cycles += 1;
        self.bpf.notify_complete(self.queue.len() as u64);
    }

    fn update_core_snapshot(&mut self) {
        let now_ns = monotonic_ns().unwrap_or_default();
        self.counters.last_snapshot_ns = now_ns;
        self.counters.scheduler_cpu_time_ns = process_cpu_time_ns()
            .unwrap_or(self.scheduler_cpu_start_ns)
            .saturating_sub(self.scheduler_cpu_start_ns);
        self.counters.core = SchedulerSnapshot {
            timestamp_ns: now_ns,
            user_dispatches: *self.bpf.nr_user_dispatches_mut(),
            kernel_dispatches: *self.bpf.nr_kernel_dispatches_mut(),
            cancelled_dispatches: *self.bpf.nr_cancel_dispatches_mut(),
            bounced_dispatches: *self.bpf.nr_bounce_dispatches_mut(),
            failed_dispatches: *self.bpf.nr_failed_dispatches_mut(),
            congestion_events: *self.bpf.nr_sched_congested_mut(),
            queued: *self.bpf.nr_queued_mut(),
            scheduled: *self.bpf.nr_scheduled_mut(),
            running: *self.bpf.nr_running_mut(),
            notify_cycles: self.counters.notify_cycles,
            tasks_submitted: self.counters.local_dispatches,
        };
    }

    fn maybe_write_stats(&mut self, force: bool) {
        if !force && self.last_stats.elapsed() < self.stats_interval {
            return;
        }
        self.update_core_snapshot();
        if let Err(err) = write_json_atomic(&self.stats_json, &self.counters) {
            tracing::warn!(error = %err, path = %self.stats_json.display(), "stats write failed");
        }
        self.last_stats = Instant::now();
    }

    fn run(&mut self) -> Result<()> {
        while !self.bpf.exited() {
            let cycle_started = monotonic_ns().unwrap_or_default();
            self.consume_available();
            self.dispatch_batch();
            let cycle_finished = monotonic_ns().unwrap_or(cycle_started);
            let cycle_wall_ns = cycle_finished.saturating_sub(cycle_started);
            self.counters.total_cycle_wall_ns = self
                .counters
                .total_cycle_wall_ns
                .saturating_add(cycle_wall_ns);
            self.counters.max_cycle_wall_ns = self.counters.max_cycle_wall_ns.max(cycle_wall_ns);
            self.maybe_write_stats(false);
        }
        self.maybe_write_stats(true);
        Ok(())
    }
}

fn relative_vtime(score_ns: i128, minimum_ns: i128) -> u64 {
    let relative = score_ns.saturating_sub(minimum_ns);
    if relative <= 0 {
        0
    } else if relative > u64::MAX as i128 {
        u64::MAX
    } else {
        relative as u64
    }
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();
    let args = Args::parse();
    let cfg = ExperimentConfig::from_path(&args.config)
        .with_context(|| format!("failed to load {}", args.config.display()))?;

    // scx_rustland_core installs and owns the Ctrl-C handler. Registering a
    // second handler here would fail on target hosts.
    loop {
        // A restart must construct a fresh OpenObject. Reusing an initialized
        // MaybeUninit across BPF reloads can retain stale skeleton state.
        let mut open_object = MaybeUninit::uninit();
        let mut scheduler = Scheduler::init(&mut open_object, &args, cfg.clone())?;
        scheduler.run()?;
        let exit = scheduler.bpf.shutdown_and_report()?;
        if !exit.should_restart() {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vtime_preserves_relative_order() {
        assert_eq!(relative_vtime(-10, -10), 0);
        assert_eq!(relative_vtime(5, -10), 15);
    }
}
