use afs_common::{
    generate_task_manifest, AdmissionDecision, ExperimentConfig, PlannedTask, SchedulerMethod,
    TaskMetadata, TaskResult,
};
use afs_policy_core::{
    decide_admission, relevant_interference_ns, score_task, AdmissionState, CpuPressure,
    PolicyObservation, PsiLine,
};
use anyhow::{bail, Context, Result};
use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(about = "Deterministic discrete-time simulator for the application-aware policy")]
struct Args {
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    manifest: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    repetition: u32,
    #[arg(long)]
    no_aging: bool,
    #[arg(long)]
    no_admission: bool,
    #[arg(long)]
    virtual_deadline: bool,
}

#[derive(Debug, Clone)]
struct PendingTask {
    plan: PlannedTask,
    next_eligible_ns: u64,
    request_arrival_ns: Option<u64>,
    delayed_ns: u64,
}

#[derive(Debug, Clone)]
struct SimTask {
    plan: PlannedTask,
    metadata: TaskMetadata,
    remaining_ns: u64,
    consumed_ns: u64,
    first_start_ns: Option<u64>,
    last_enqueue_ns: u64,
    delayed_ns: u64,
    admission: AdmissionDecision,
}

fn estimate_relevant_ready_work(
    ready: &[SimTask],
    incoming: &TaskMetadata,
    now_ns: u64,
    cfg: &ExperimentConfig,
) -> u64 {
    let incoming_virtual_deadline = if cfg.policy.application_deadline {
        None
    } else {
        Some(
            incoming
                .release_time_ns
                .saturating_add(incoming.estimated_remaining_runtime_ns),
        )
    };

    let incoming_obs = PolicyObservation {
        metadata: incoming.clone(),
        now_ns,
        consumed_cpu_ns: 0,
        last_enqueue_ns: now_ns,
        virtual_deadline_ns: incoming_virtual_deadline,
    };

    let observations: Vec<(PolicyObservation, u64)> = ready
        .iter()
        .map(|task| {
            let virtual_deadline = if cfg.policy.application_deadline {
                None
            } else {
                Some(
                    task.metadata
                        .release_time_ns
                        .saturating_add(task.consumed_ns)
                        .saturating_add(task.plan.actual_runtime_ns),
                )
            };

            let obs = PolicyObservation {
                metadata: task.metadata.clone(),
                now_ns,
                consumed_cpu_ns: task.consumed_ns,
                last_enqueue_ns: task.last_enqueue_ns,
                virtual_deadline_ns: virtual_deadline,
            };

            (obs, task.remaining_ns)
        })
        .collect();

    relevant_interference_ns(&incoming_obs, &observations, &cfg.policy)
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();
    let args = Args::parse();
    let mut cfg = ExperimentConfig::from_path(&args.config)?;
    cfg.experiment.scheduler = SchedulerMethod::Simulator;
    if args.no_aging {
        cfg.policy.aging = false;
    }
    if args.no_admission {
        cfg.admission.enabled = false;
    }
    if args.virtual_deadline {
        cfg.policy.application_deadline = false;
    }

    let plans = match &args.manifest {
        Some(path) => PlannedTask::read_manifest(path)?,
        None => generate_task_manifest(&cfg, args.repetition)?,
    };
    if let Some(path) = &args.manifest {
        tracing::info!(manifest = %path.display(), tasks = plans.len(), "using existing manifest");
    }
    let results = simulate(&cfg, &plans, args.repetition)?;
    write_results(&args.output, &results)?;

    let admitted = results
        .iter()
        .filter(|r| r.admission != AdmissionDecision::Reject)
        .count();
    let rejected = results.len().saturating_sub(admitted);
    let completed = results
        .iter()
        .filter(|r| r.completion_time_ns.is_some())
        .count();
    let missed = results
        .iter()
        .filter(|r| r.deadline_missed == Some(true))
        .count();
    println!(
        "tasks={} admitted={} rejected={} completed={} missed={} output={}",
        results.len(),
        admitted,
        rejected,
        completed,
        missed,
        args.output.display()
    );
    Ok(())
}

fn simulate(
    cfg: &ExperimentConfig,
    plans: &[PlannedTask],
    repetition: u32,
) -> Result<Vec<TaskResult>> {
    let quantum_ns = cfg.policy.quantum_ms.saturating_mul(1_000_000).max(1);
    let cpus = cfg.experiment.cpus.max(1);
    let mut pending: Vec<PendingTask> = plans
        .iter()
        .cloned()
        .map(|plan| PendingTask {
            next_eligible_ns: plan.release_offset_ns,
            request_arrival_ns: None,
            delayed_ns: 0,
            plan,
        })
        .collect();
    let mut ready: Vec<SimTask> = Vec::new();
    let mut results: Vec<TaskResult> = Vec::with_capacity(plans.len());
    let mut outcomes: HashMap<u64, bool> = HashMap::new();
    let mut completion_times: HashMap<u64, u64> = HashMap::new();
    let mut now_ns = 0u64;
    let mut safety_ticks = 0u64;
    let total_work: u64 = plans.iter().map(|p| p.actual_runtime_ns).sum();
    let last_release = plans.iter().map(|p| p.release_offset_ns).max().unwrap_or(0);
    let hard_limit_ns = cfg
        .experiment
        .duration_s
        .saturating_mul(1_000_000_000)
        .max(last_release.saturating_add(total_work.saturating_mul(20).max(1)));

    while !pending.is_empty() || !ready.is_empty() {
        let released = release_tasks(
            cfg,
            &mut pending,
            &mut ready,
            &mut results,
            &mut outcomes,
            &completion_times,
            now_ns,
            repetition,
        )?;

        if ready.is_empty() {
            if pending.is_empty() {
                break;
            }
            let next = pending
                .iter()
                .filter(|p| dependency_resolved(&p.plan, &outcomes))
                .map(|p| p.next_eligible_ns)
                .filter(|t| *t > now_ns)
                .min();
            match next {
                Some(next_ns) => {
                    now_ns = next_ns;
                    continue;
                }
                None if released => continue,
                None => bail!("simulation dependency deadlock or cyclic manifest"),
            }
        }

        let mut ranked: Vec<(usize, i128)> = ready
            .iter()
            .enumerate()
            .map(|(idx, task)| {
                // Rustland-style ablation: a virtual deadline derived from prior
                // virtual progress and the current CPU burst, not an application SLA.
                let virtual_deadline = Some(
                    task.metadata
                        .release_time_ns
                        .saturating_add(task.consumed_ns)
                        .saturating_add(task.plan.actual_runtime_ns),
                );
                let obs = PolicyObservation {
                    metadata: task.metadata.clone(),
                    now_ns,
                    consumed_cpu_ns: task.consumed_ns,
                    last_enqueue_ns: task.last_enqueue_ns,
                    virtual_deadline_ns: virtual_deadline,
                };
                (idx, score_task(&obs, &cfg.policy).score_ns)
            })
            .collect();
        ranked.sort_by_key(|(idx, score)| (*score, ready[*idx].plan.task_id));
        let selected: Vec<usize> = ranked
            .into_iter()
            .take(cpus.min(ready.len()))
            .map(|(idx, _)| idx)
            .collect();

        let step_ns = selected
            .iter()
            .map(|idx| ready[*idx].remaining_ns)
            .min()
            .unwrap_or(quantum_ns)
            .min(quantum_ns)
            .max(1);
        let next_now = now_ns.saturating_add(step_ns);

        for idx in &selected {
            let task = &mut ready[*idx];
            task.first_start_ns.get_or_insert(now_ns);
            task.remaining_ns = task.remaining_ns.saturating_sub(step_ns);
            task.consumed_ns = task.consumed_ns.saturating_add(step_ns);
            task.metadata.estimated_remaining_runtime_ns = task
                .metadata
                .estimated_total_runtime_ns
                .saturating_sub(task.consumed_ns);
            if task.remaining_ns > 0 {
                task.last_enqueue_ns = next_now;
            }
        }
        now_ns = next_now;

        let mut completed_indices: Vec<usize> = selected
            .into_iter()
            .filter(|idx| ready[*idx].remaining_ns == 0)
            .collect();
        completed_indices.sort_unstable_by(|a, b| b.cmp(a));
        for idx in completed_indices {
            let task = ready.swap_remove(idx);
            let result = completed_result(cfg, task, now_ns, repetition);
            outcomes.insert(result.task_id, true);
            completion_times.insert(result.task_id, now_ns);
            results.push(result);
        }

        safety_ticks = safety_ticks.saturating_add(1);
        if now_ns > hard_limit_ns || safety_ticks > 100_000_000 {
            bail!("simulation exceeded safety limit; check configuration");
        }
    }

    results.sort_by_key(|r| r.task_id);
    Ok(results)
}

#[allow(clippy::too_many_arguments)]
fn release_tasks(
    cfg: &ExperimentConfig,
    pending: &mut Vec<PendingTask>,
    ready: &mut Vec<SimTask>,
    results: &mut Vec<TaskResult>,
    outcomes: &mut HashMap<u64, bool>,
    completion_times: &HashMap<u64, u64>,
    now_ns: u64,
    repetition: u32,
) -> Result<bool> {
    let mut changed = false;
    let mut idx = 0usize;
    while idx < pending.len() {
        if pending[idx].next_eligible_ns > now_ns {
            idx += 1;
            continue;
        }
        if let Some(parent) = pending[idx].plan.depends_on_task_id {
            match outcomes.get(&parent).copied() {
                None => {
                    idx += 1;
                    continue;
                }
                Some(false) => {
                    let pending_task = pending.swap_remove(idx);
                    let arrival = pending_task
                        .request_arrival_ns
                        .unwrap_or(pending_task.plan.release_offset_ns);
                    let deadline = pending_task
                        .plan
                        .relative_deadline_ns
                        .map(|d| arrival.saturating_add(d));
                    let mut result = TaskResult::rejected(
                        cfg.experiment.name.clone(),
                        repetition,
                        SchedulerMethod::Simulator,
                        pending_task.plan.task_id,
                        pending_task.plan.workflow_id,
                        pending_task.plan.stage_id,
                        pending_task.plan.name,
                        pending_task.plan.workload_class,
                        pending_task.plan.kind,
                        arrival,
                        deadline,
                        pending_task.plan.estimated_runtime_ns,
                        pending_task.delayed_ns,
                    );
                    result.error = Some(format!("dependency task {parent} failed or was rejected"));
                    outcomes.insert(result.task_id, false);
                    results.push(result);
                    changed = true;
                    continue;
                }
                Some(true) => {}
            }
        }

        let mut pending_task = pending.swap_remove(idx);
        let dependency_ready = pending_task
            .plan
            .depends_on_task_id
            .and_then(|id| completion_times.get(&id).copied())
            .unwrap_or(pending_task.plan.release_offset_ns);
        let request_arrival_ns = *pending_task
            .request_arrival_ns
            .get_or_insert(pending_task.plan.release_offset_ns.max(dependency_ready));
        let deadline = pending_task
            .plan
            .relative_deadline_ns
            .map(|d| request_arrival_ns.saturating_add(d));
        let metadata = TaskMetadata {
            task_id: pending_task.plan.task_id,
            process_id: pending_task.plan.task_id as i32,
            workflow_id: pending_task.plan.workflow_id,
            stage_id: pending_task.plan.stage_id,
            workload_class: pending_task.plan.workload_class,
            kind: pending_task.plan.kind,
            release_time_ns: request_arrival_ns,
            absolute_deadline_ns: deadline,
            estimated_total_runtime_ns: pending_task.plan.estimated_runtime_ns,
            estimated_remaining_runtime_ns: pending_task.plan.estimated_runtime_ns,
            application_priority: pending_task.plan.application_priority,
            name: pending_task.plan.name.clone(),
        };
        let synthetic_psi = if ready.len() <= cfg.experiment.cpus {
            0.0
        } else {
            ((ready.len() - cfg.experiment.cpus) as f64 / ready.len() as f64) * 100.0
        };
        let mut admission_metadata = metadata.clone();
        if !cfg.policy.application_deadline {
            // Preserve the real application deadline for evaluation, but do
            // not expose it to the virtual-deadline ablation's admission path.
            admission_metadata.absolute_deadline_ns = None;
        }
        let active_work = estimate_relevant_ready_work(&ready, &admission_metadata, now_ns, &cfg);
        let admission = decide_admission(
            &admission_metadata,
            now_ns,
            AdmissionState {
                active_estimated_remaining_ns: active_work,
                active_tasks: ready.len(),
                cpus: cfg.experiment.cpus,
                pressure: CpuPressure {
                    some: PsiLine {
                        avg10: synthetic_psi,
                        ..Default::default()
                    },
                    full: None,
                },
            },
            &cfg.admission,
        );

        match admission.decision {
            AdmissionDecision::Admit => {
                let final_admission = if pending_task.delayed_ns == 0 {
                    AdmissionDecision::Admit
                } else {
                    AdmissionDecision::Delay
                };
                ready.push(SimTask {
                    remaining_ns: pending_task.plan.actual_runtime_ns,
                    consumed_ns: 0,
                    first_start_ns: None,
                    last_enqueue_ns: now_ns,
                    delayed_ns: pending_task.delayed_ns,
                    admission: final_admission,
                    metadata,
                    plan: pending_task.plan,
                });
                changed = true;
            }
            AdmissionDecision::Delay => {
                let step = cfg.admission.delay_step_ms.saturating_mul(1_000_000);
                let max = cfg.admission.max_delay_ms.saturating_mul(1_000_000);
                if step == 0 || pending_task.delayed_ns.saturating_add(step) > max {
                    let mut result = TaskResult::rejected(
                        cfg.experiment.name.clone(),
                        repetition,
                        SchedulerMethod::Simulator,
                        pending_task.plan.task_id,
                        pending_task.plan.workflow_id,
                        pending_task.plan.stage_id,
                        pending_task.plan.name,
                        pending_task.plan.workload_class,
                        pending_task.plan.kind,
                        request_arrival_ns,
                        deadline,
                        pending_task.plan.estimated_runtime_ns,
                        pending_task.delayed_ns,
                    );
                    result.error = Some("maximum admission delay exceeded".into());
                    outcomes.insert(result.task_id, false);
                    results.push(result);
                } else {
                    pending_task.delayed_ns = pending_task.delayed_ns.saturating_add(step);
                    pending_task.next_eligible_ns = now_ns.saturating_add(step);
                    pending.push(pending_task);
                }
                changed = true;
            }
            AdmissionDecision::Reject => {
                let mut result = TaskResult::rejected(
                    cfg.experiment.name.clone(),
                    repetition,
                    SchedulerMethod::Simulator,
                    pending_task.plan.task_id,
                    pending_task.plan.workflow_id,
                    pending_task.plan.stage_id,
                    pending_task.plan.name,
                    pending_task.plan.workload_class,
                    pending_task.plan.kind,
                    request_arrival_ns,
                    deadline,
                    pending_task.plan.estimated_runtime_ns,
                    pending_task.delayed_ns,
                );
                result.error = Some(admission.reason.into());
                outcomes.insert(result.task_id, false);
                results.push(result);
                changed = true;
            }
        }
    }
    Ok(changed)
}

fn dependency_resolved(plan: &PlannedTask, outcomes: &HashMap<u64, bool>) -> bool {
    plan.depends_on_task_id
        .map(|id| outcomes.contains_key(&id))
        .unwrap_or(true)
}

fn completed_result(
    cfg: &ExperimentConfig,
    task: SimTask,
    completion_ns: u64,
    repetition: u32,
) -> TaskResult {
    let response = completion_ns.saturating_sub(task.metadata.release_time_ns);
    let missed = task
        .metadata
        .absolute_deadline_ns
        .map(|d| completion_ns > d);
    TaskResult {
        experiment: cfg.experiment.name.clone(),
        repetition,
        scheduler: SchedulerMethod::Simulator,
        task_id: task.plan.task_id,
        process_id: task.plan.task_id as i32,
        workflow_id: task.plan.workflow_id,
        stage_id: task.plan.stage_id,
        task_name: task.plan.name,
        workload_class: task.plan.workload_class,
        kind: task.plan.kind,
        admission: task.admission,
        delayed_ns: task.delayed_ns,
        release_time_ns: task.metadata.release_time_ns,
        start_time_ns: task.first_start_ns,
        completion_time_ns: Some(completion_ns),
        absolute_deadline_ns: task.metadata.absolute_deadline_ns,
        estimated_runtime_ns: task.plan.estimated_runtime_ns,
        actual_cpu_time_ns: Some(task.consumed_ns),
        response_time_ns: Some(response),
        deadline_missed: missed,
        exit_code: Some(0),
        error: None,
    }
}

fn write_results(path: &Path, results: &[TaskResult]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut writer = csv::Writer::from_path(path)
        .with_context(|| format!("failed to create {}", path.display()))?;
    for result in results {
        writer.serialize(result)?;
    }
    writer.flush()?;
    Ok(())
}
