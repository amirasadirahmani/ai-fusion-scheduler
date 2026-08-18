use crate::CpuPressure;
use afs_common::{AdmissionConfig, AdmissionDecision, TaskMetadata, WorkloadClass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AdmissionState {
    pub active_estimated_remaining_ns: u64,
    pub active_tasks: usize,
    pub cpus: usize,
    pub pressure: CpuPressure,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AdmissionOutcome {
    pub decision: AdmissionDecision,
    pub predicted_finish_ns: u64,
    pub available_until_deadline_ns: Option<u64>,
    pub reason: &'static str,
}

pub fn decide_admission(
    task: &TaskMetadata,
    now_ns: u64,
    state: AdmissionState,
    cfg: &AdmissionConfig,
) -> AdmissionOutcome {
    let cpus = state.cpus.max(1) as u64;
    let parallel_interference = state.active_estimated_remaining_ns / cpus;
    let base_duration = task
        .estimated_remaining_runtime_ns
        .saturating_add(parallel_interference);
    let predicted_duration = (base_duration as f64 * cfg.safety_factor.max(1.0)).ceil() as u64;
    let predicted_finish_ns = now_ns.saturating_add(predicted_duration);
    let available = task
        .absolute_deadline_ns
        .map(|deadline| deadline.saturating_sub(now_ns));

    if !cfg.enabled {
        return AdmissionOutcome {
            decision: AdmissionDecision::Admit,
            predicted_finish_ns,
            available_until_deadline_ns: available,
            reason: "admission_disabled",
        };
    }

    if let Some(deadline) = task.absolute_deadline_ns {
        if deadline <= now_ns {
            return AdmissionOutcome {
                decision: AdmissionDecision::Reject,
                predicted_finish_ns,
                available_until_deadline_ns: available,
                reason: "deadline_already_expired",
            };
        }
        if predicted_finish_ns > deadline {
            return AdmissionOutcome {
                decision: AdmissionDecision::Reject,
                predicted_finish_ns,
                available_until_deadline_ns: available,
                reason: "predicted_infeasible",
            };
        }
    }

    let psi = state.pressure.some.avg10;
    if psi >= cfg.psi_reject_threshold
        && task.application_priority < cfg.reject_below_priority
        && task.workload_class != WorkloadClass::Critical
    {
        return AdmissionOutcome {
            decision: AdmissionDecision::Reject,
            predicted_finish_ns,
            available_until_deadline_ns: available,
            reason: "cpu_pressure_reject",
        };
    }

    if psi >= cfg.psi_delay_threshold && task.workload_class != WorkloadClass::Critical {
        let delay_step_ns = cfg.delay_step_ms.saturating_mul(1_000_000);

        // Deadline-aware deferral:
        //
        // Do not deliberately delay a task that is feasible now if the next
        // admission delay step would consume the remaining deadline slack and
        // make that same task infeasible under the current prediction.
        //
        // Under severe PSI the earlier cpu_pressure_reject branch still wins.
        if delay_step_ns > 0 {
            if let Some(deadline) = task.absolute_deadline_ns {
                let predicted_after_delay = predicted_finish_ns.saturating_add(delay_step_ns);

                if predicted_after_delay > deadline {
                    return AdmissionOutcome {
                        decision: AdmissionDecision::Admit,
                        predicted_finish_ns,
                        available_until_deadline_ns: available,
                        reason: "deadline_slack_exhausted_admit",
                    };
                }
            }
        }

        return AdmissionOutcome {
            decision: AdmissionDecision::Delay,
            predicted_finish_ns,
            available_until_deadline_ns: available,
            reason: "cpu_pressure_delay",
        };
    }

    AdmissionOutcome {
        decision: AdmissionDecision::Admit,
        predicted_finish_ns,
        available_until_deadline_ns: available,
        reason: "feasible",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use afs_common::{AdmissionConfig, TaskMetadata, WorkloadKind};

    fn task(deadline: Option<u64>) -> TaskMetadata {
        TaskMetadata {
            task_id: 1,
            process_id: 1,
            workflow_id: 0,
            stage_id: 0,
            workload_class: WorkloadClass::Interactive,
            kind: WorkloadKind::Cpu,
            release_time_ns: 0,
            absolute_deadline_ns: deadline,
            estimated_total_runtime_ns: 20,
            estimated_remaining_runtime_ns: 20,
            application_priority: 0.5,
            name: "x".into(),
        }
    }

    #[test]
    fn rejects_infeasible_deadline() {
        let out = decide_admission(
            &task(Some(10)),
            0,
            AdmissionState {
                active_estimated_remaining_ns: 100,
                cpus: 1,
                ..AdmissionState::default()
            },
            &AdmissionConfig::default(),
        );
        assert_eq!(out.decision, AdmissionDecision::Reject);
    }

    #[test]
    fn critical_is_not_psi_rejected() {
        let mut t = task(None);
        t.workload_class = WorkloadClass::Critical;
        let out = decide_admission(
            &t,
            0,
            AdmissionState {
                pressure: CpuPressure {
                    some: crate::PsiLine {
                        avg10: 99.0,
                        ..Default::default()
                    },
                    full: None,
                },
                cpus: 1,
                ..AdmissionState::default()
            },
            &AdmissionConfig::default(),
        );
        assert_eq!(out.decision, AdmissionDecision::Admit);
    }
    #[test]
    fn own_runtime_is_not_parallelized_across_cpus() {
        let mut t = task(Some(50));
        t.estimated_total_runtime_ns = 100;
        t.estimated_remaining_runtime_ns = 100;

        let out = decide_admission(
            &t,
            0,
            AdmissionState {
                active_estimated_remaining_ns: 0,
                cpus: 4,
                ..AdmissionState::default()
            },
            &AdmissionConfig::default(),
        );

        assert_eq!(out.decision, AdmissionDecision::Reject);
    }

    #[test]
    fn pressure_delay_admits_when_next_step_would_exhaust_deadline_slack() {
        let mut t = task(Some(100_000_000));
        t.estimated_total_runtime_ns = 70_000_000;
        t.estimated_remaining_runtime_ns = 70_000_000;

        let mut cfg = AdmissionConfig::default();
        cfg.enabled = true;
        cfg.safety_factor = 1.20;
        cfg.psi_delay_threshold = 20.0;
        cfg.psi_reject_threshold = 60.0;
        cfg.delay_step_ms = 25;

        let out = decide_admission(
            &t,
            0,
            AdmissionState {
                pressure: CpuPressure {
                    some: crate::PsiLine {
                        avg10: 30.0,
                        ..Default::default()
                    },
                    full: None,
                },
                cpus: 4,
                ..AdmissionState::default()
            },
            &cfg,
        );

        assert_eq!(out.decision, AdmissionDecision::Admit);
        assert_eq!(out.reason, "deadline_slack_exhausted_admit");
    }

    #[test]
    fn pressure_delay_continues_when_deadline_has_sufficient_slack() {
        let mut t = task(Some(150_000_000));
        t.estimated_total_runtime_ns = 70_000_000;
        t.estimated_remaining_runtime_ns = 70_000_000;

        let mut cfg = AdmissionConfig::default();
        cfg.enabled = true;
        cfg.safety_factor = 1.20;
        cfg.psi_delay_threshold = 20.0;
        cfg.psi_reject_threshold = 60.0;
        cfg.delay_step_ms = 25;

        let out = decide_admission(
            &t,
            0,
            AdmissionState {
                pressure: CpuPressure {
                    some: crate::PsiLine {
                        avg10: 30.0,
                        ..Default::default()
                    },
                    full: None,
                },
                cpus: 4,
                ..AdmissionState::default()
            },
            &cfg,
        );

        assert_eq!(out.decision, AdmissionDecision::Delay);
        assert_eq!(out.reason, "cpu_pressure_delay");
    }
}
