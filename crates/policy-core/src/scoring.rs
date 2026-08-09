use afs_common::{PolicyConfig, TaskMetadata, WorkloadClass};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyObservation {
    pub metadata: TaskMetadata,
    pub now_ns: u64,
    pub consumed_cpu_ns: u64,
    pub last_enqueue_ns: u64,
    /// Optional rustland-like virtual deadline used by the ablation.
    pub virtual_deadline_ns: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub remaining_ns: u64,
    pub waiting_ns: u64,
    pub laxity_ns: i128,
    pub priority_bonus_ns: i128,
    pub aging_bonus_ns: i128,
    pub score_ns: i128,
}

pub fn normalize_priority(priority: f64) -> f64 {
    priority.clamp(0.0, 1.0)
}

pub fn normalize_aging(waiting_ns: u64, horizon_ns: u64) -> f64 {
    if horizon_ns == 0 {
        return 1.0;
    }
    (waiting_ns as f64 / horizon_ns as f64).clamp(0.0, 1.0)
}

pub fn score_task(obs: &PolicyObservation, cfg: &PolicyConfig) -> ScoreBreakdown {
    let remaining_ns = obs
        .metadata
        .estimated_total_runtime_ns
        .saturating_sub(obs.consumed_cpu_ns)
        .min(obs.metadata.estimated_remaining_runtime_ns.max(1));
    let waiting_ns = obs.now_ns.saturating_sub(obs.last_enqueue_ns);

    let laxity_ns = if cfg.application_deadline {
        match obs.metadata.absolute_deadline_ns {
            Some(deadline) => deadline as i128 - obs.now_ns as i128 - remaining_ns as i128,
            None => {
                // Batch/no-deadline work starts with a deliberately relaxed
                // soft laxity and can move forward through explicit aging.
                cfg.batch_base_laxity_ms as i128 * 1_000_000
            }
        }
    } else {
        // A rustland-style virtual deadline lives on a virtual runtime axis,
        // not the CLOCK_MONOTONIC axis. It must therefore be ranked directly;
        // subtracting now_ns would mix unrelated clocks and invalidate the
        // virtual-deadline ablation.
        obs.virtual_deadline_ns
            .map(i128::from)
            .unwrap_or(cfg.batch_base_laxity_ms as i128 * 1_000_000)
    };

    let priority_bonus_ns = if cfg.priority {
        (cfg.alpha_ms * 1_000_000.0 * normalize_priority(obs.metadata.application_priority)).round()
            as i128
    } else {
        0
    };

    let aging_bonus_ns = if cfg.aging {
        let age = normalize_aging(waiting_ns, cfg.aging_horizon_ms.saturating_mul(1_000_000));
        (cfg.beta_ms * 1_000_000.0 * age).round() as i128
    } else {
        0
    };

    ScoreBreakdown {
        remaining_ns,
        waiting_ns,
        laxity_ns,
        priority_bonus_ns,
        aging_bonus_ns,
        score_ns: laxity_ns - priority_bonus_ns - aging_bonus_ns,
    }
}

pub fn class_slice_ns(class: WorkloadClass, cfg: &PolicyConfig) -> u64 {
    let ms = match class {
        WorkloadClass::Critical => cfg.critical_slice_ms,
        WorkloadClass::Interactive => cfg.interactive_slice_ms,
        WorkloadClass::Batch => cfg.batch_slice_ms,
    };
    ms.max(1).saturating_mul(1_000_000)
}

/// Deterministic comparison: score, release, task_id.
pub fn compare_tasks(
    a: (&PolicyObservation, &ScoreBreakdown),
    b: (&PolicyObservation, &ScoreBreakdown),
) -> Ordering {
    a.1.score_ns
        .cmp(&b.1.score_ns)
        .then_with(|| {
            a.0.metadata
                .release_time_ns
                .cmp(&b.0.metadata.release_time_ns)
        })
        .then_with(|| a.0.metadata.task_id.cmp(&b.0.metadata.task_id))
}

/// Sum remaining work from active tasks that currently rank no later
/// than the incoming task under the same policy used by the scheduler.
pub fn relevant_interference_ns(
    incoming: &PolicyObservation,
    active: &[(PolicyObservation, u64)],
    cfg: &PolicyConfig,
) -> u64 {
    let incoming_score = score_task(incoming, cfg);

    active
        .iter()
        .filter_map(|(obs, remaining_ns)| {
            let active_score = score_task(obs, cfg);
            match compare_tasks((obs, &active_score), (incoming, &incoming_score)) {
                Ordering::Less | Ordering::Equal => Some(*remaining_ns),
                Ordering::Greater => None,
            }
        })
        .fold(0u64, u64::saturating_add)
}

#[cfg(test)]
mod tests {
    use super::*;
    use afs_common::{TaskMetadata, WorkloadKind};

    fn task(id: u64, deadline: Option<u64>, priority: f64) -> TaskMetadata {
        TaskMetadata {
            task_id: id,
            process_id: id as i32,
            workflow_id: 0,
            stage_id: 0,
            workload_class: WorkloadClass::Interactive,
            kind: WorkloadKind::Cpu,
            release_time_ns: 0,
            absolute_deadline_ns: deadline,
            estimated_total_runtime_ns: 20,
            estimated_remaining_runtime_ns: 20,
            application_priority: priority,
            name: format!("t{id}"),
        }
    }

    #[test]
    fn lower_laxity_wins() {
        let cfg = PolicyConfig {
            alpha_ms: 0.0,
            beta_ms: 0.0,
            ..PolicyConfig::default()
        };
        let a = PolicyObservation {
            metadata: task(1, Some(100), 0.0),
            now_ns: 10,
            consumed_cpu_ns: 0,
            last_enqueue_ns: 0,
            virtual_deadline_ns: None,
        };
        let b = PolicyObservation {
            metadata: task(2, Some(200), 0.0),
            now_ns: 10,
            consumed_cpu_ns: 0,
            last_enqueue_ns: 0,
            virtual_deadline_ns: None,
        };
        assert!(score_task(&a, &cfg).score_ns < score_task(&b, &cfg).score_ns);
    }

    #[test]
    fn aging_improves_waiting_task() {
        let cfg = PolicyConfig {
            alpha_ms: 0.0,
            beta_ms: 100.0,
            aging_horizon_ms: 100,
            ..PolicyConfig::default()
        };
        let fresh = PolicyObservation {
            metadata: task(1, None, 0.0),
            now_ns: 100_000_000,
            consumed_cpu_ns: 0,
            last_enqueue_ns: 100_000_000,
            virtual_deadline_ns: None,
        };
        let old = PolicyObservation {
            metadata: task(2, None, 0.0),
            now_ns: 100_000_000,
            consumed_cpu_ns: 0,
            last_enqueue_ns: 0,
            virtual_deadline_ns: None,
        };
        assert!(score_task(&old, &cfg).score_ns < score_task(&fresh, &cfg).score_ns);
    }

    #[test]
    fn virtual_deadline_is_not_mixed_with_monotonic_time() {
        let cfg = PolicyConfig {
            application_deadline: false,
            alpha_ms: 0.0,
            beta_ms: 0.0,
            ..PolicyConfig::default()
        };
        let obs = PolicyObservation {
            metadata: task(1, Some(1), 0.0),
            now_ns: 9_000_000_000_000,
            consumed_cpu_ns: 0,
            last_enqueue_ns: 0,
            virtual_deadline_ns: Some(1234),
        };
        assert_eq!(score_task(&obs, &cfg).laxity_ns, 1234);
    }
    #[test]
    fn low_priority_batch_does_not_interfere_with_urgent_critical() {
        let cfg = PolicyConfig {
            alpha_ms: 50.0,
            beta_ms: 0.0,
            application_deadline: true,
            ..PolicyConfig::default()
        };

        let incoming = PolicyObservation {
            metadata: TaskMetadata {
                task_id: 2,
                process_id: 2,
                workflow_id: 0,
                stage_id: 0,
                workload_class: WorkloadClass::Critical,
                kind: WorkloadKind::Cpu,
                release_time_ns: 50_000_000,
                absolute_deadline_ns: Some(170_000_000),
                estimated_total_runtime_ns: 25_000_000,
                estimated_remaining_runtime_ns: 25_000_000,
                application_priority: 1.0,
                name: "critical".into(),
            },
            now_ns: 50_000_000,
            consumed_cpu_ns: 0,
            last_enqueue_ns: 50_000_000,
            virtual_deadline_ns: None,
        };

        let batch = PolicyObservation {
            metadata: TaskMetadata {
                task_id: 1,
                process_id: 1,
                workflow_id: 0,
                stage_id: 0,
                workload_class: WorkloadClass::Batch,
                kind: WorkloadKind::Cpu,
                release_time_ns: 0,
                absolute_deadline_ns: None,
                estimated_total_runtime_ns: 100_000_000,
                estimated_remaining_runtime_ns: 100_000_000,
                application_priority: 0.15,
                name: "batch".into(),
            },
            now_ns: 50_000_000,
            consumed_cpu_ns: 0,
            last_enqueue_ns: 0,
            virtual_deadline_ns: None,
        };

        let active = vec![(batch, 100_000_000)];
        assert_eq!(relevant_interference_ns(&incoming, &active, &cfg), 0);
    }
}
