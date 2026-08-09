use crate::{AdmissionDecision, SchedulerMethod, WorkloadClass, WorkloadKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub experiment: String,
    pub repetition: u32,
    pub scheduler: SchedulerMethod,
    pub task_id: u64,
    pub process_id: i32,
    pub workflow_id: u64,
    pub stage_id: u32,
    pub task_name: String,
    pub workload_class: WorkloadClass,
    pub kind: WorkloadKind,
    pub admission: AdmissionDecision,
    pub delayed_ns: u64,
    pub release_time_ns: u64,
    pub start_time_ns: Option<u64>,
    pub completion_time_ns: Option<u64>,
    pub absolute_deadline_ns: Option<u64>,
    pub estimated_runtime_ns: u64,
    pub actual_cpu_time_ns: Option<u64>,
    pub response_time_ns: Option<u64>,
    pub deadline_missed: Option<bool>,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
}

impl TaskResult {
    pub fn rejected(
        experiment: String,
        repetition: u32,
        scheduler: SchedulerMethod,
        task_id: u64,
        workflow_id: u64,
        stage_id: u32,
        task_name: String,
        workload_class: WorkloadClass,
        kind: WorkloadKind,
        release_time_ns: u64,
        absolute_deadline_ns: Option<u64>,
        estimated_runtime_ns: u64,
        delayed_ns: u64,
    ) -> Self {
        Self {
            experiment,
            repetition,
            scheduler,
            task_id,
            process_id: -1,
            workflow_id,
            stage_id,
            task_name,
            workload_class,
            kind,
            admission: AdmissionDecision::Reject,
            delayed_ns,
            release_time_ns,
            start_time_ns: None,
            completion_time_ns: None,
            absolute_deadline_ns,
            estimated_runtime_ns,
            actual_cpu_time_ns: None,
            response_time_ns: None,
            deadline_missed: None,
            exit_code: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulerSnapshot {
    pub timestamp_ns: u64,
    pub user_dispatches: u64,
    pub kernel_dispatches: u64,
    pub cancelled_dispatches: u64,
    pub bounced_dispatches: u64,
    pub failed_dispatches: u64,
    pub congestion_events: u64,
    pub queued: u64,
    pub scheduled: u64,
    pub running: u64,
    pub notify_cycles: u64,
    pub tasks_submitted: u64,
}

pub fn jains_fairness(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: f64 = values.iter().sum();
    let sq_sum: f64 = values.iter().map(|v| v * v).sum();
    if sq_sum == 0.0 {
        return None;
    }
    Some((sum * sum) / (values.len() as f64 * sq_sum))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_fairness_is_one() {
        let value = jains_fairness(&[10.0, 10.0, 10.0]).unwrap();
        assert!((value - 1.0).abs() < 1e-12);
    }
}
