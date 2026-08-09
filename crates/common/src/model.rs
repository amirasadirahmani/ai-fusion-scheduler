use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadClass {
    Critical,
    Interactive,
    Batch,
}

impl Default for WorkloadClass {
    fn default() -> Self {
        Self::Interactive
    }
}

impl fmt::Display for WorkloadClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Critical => "critical",
            Self::Interactive => "interactive",
            Self::Batch => "batch",
        };
        f.write_str(text)
    }
}

impl FromStr for WorkloadClass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "critical" => Ok(Self::Critical),
            "interactive" => Ok(Self::Interactive),
            "batch" => Ok(Self::Batch),
            other => Err(format!("unknown workload class: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadKind {
    Cpu,
    Memory,
    Io,
    Mixed,
}

impl Default for WorkloadKind {
    fn default() -> Self {
        Self::Cpu
    }
}

impl fmt::Display for WorkloadKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::Io => "io",
            Self::Mixed => "mixed",
        };
        f.write_str(text)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub task_id: u64,
    pub process_id: i32,
    #[serde(default)]
    pub workflow_id: u64,
    #[serde(default)]
    pub stage_id: u32,
    #[serde(default)]
    pub workload_class: WorkloadClass,
    #[serde(default)]
    pub kind: WorkloadKind,
    /// CLOCK_MONOTONIC absolute nanoseconds.
    pub release_time_ns: u64,
    /// CLOCK_MONOTONIC absolute nanoseconds. None means no application deadline.
    #[serde(default)]
    pub absolute_deadline_ns: Option<u64>,
    /// Estimated total CPU time in nanoseconds.
    pub estimated_total_runtime_ns: u64,
    /// Estimated remaining CPU time in nanoseconds.
    pub estimated_remaining_runtime_ns: u64,
    /// Normalized priority in [0, 1].
    pub application_priority: f64,
    #[serde(default)]
    pub name: String,
}

impl TaskMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if self.process_id < 0 {
            return Err("process_id must be non-negative".into());
        }
        if !(0.0..=1.0).contains(&self.application_priority) {
            return Err("application_priority must be in [0, 1]".into());
        }
        if self.estimated_total_runtime_ns == 0 {
            return Err("estimated_total_runtime_ns must be positive".into());
        }
        if self.estimated_remaining_runtime_ns > self.estimated_total_runtime_ns {
            return Err("remaining runtime cannot exceed total runtime".into());
        }
        if let Some(deadline) = self.absolute_deadline_ns {
            if deadline <= self.release_time_ns {
                return Err("absolute deadline must be after release time".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionDecision {
    Admit,
    Delay,
    Reject,
}

impl Default for AdmissionDecision {
    fn default() -> Self {
        Self::Admit
    }
}

impl fmt::Display for AdmissionDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admit => f.write_str("admit"),
            Self::Delay => f.write_str("delay"),
            Self::Reject => f.write_str("reject"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerMethod {
    Eevdf,
    SchedDeadline,
    Rustland,
    Proposed,
    Simulator,
}

impl Default for SchedulerMethod {
    fn default() -> Self {
        Self::Eevdf
    }
}

impl fmt::Display for SchedulerMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Eevdf => "eevdf",
            Self::SchedDeadline => "sched_deadline",
            Self::Rustland => "rustland",
            Self::Proposed => "proposed",
            Self::Simulator => "simulator",
        };
        f.write_str(s)
    }
}

impl FromStr for SchedulerMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "eevdf" | "fair" | "default" => Ok(Self::Eevdf),
            "sched_deadline" | "deadline" => Ok(Self::SchedDeadline),
            "rustland" | "scx_rustland" => Ok(Self::Rustland),
            "proposed" | "application_aware" => Ok(Self::Proposed),
            "simulator" | "sim" => Ok(Self::Simulator),
            other => Err(format!("unknown scheduler method: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerLaunchSpec {
    pub experiment: String,
    pub repetition: u32,
    pub scheduler: SchedulerMethod,
    #[serde(default)]
    pub admission: AdmissionDecision,
    pub task_id: u64,
    pub task_name: String,
    pub workflow_id: u64,
    pub stage_id: u32,
    pub workload_class: WorkloadClass,
    pub kind: WorkloadKind,
    pub estimated_runtime_ns: u64,
    pub actual_runtime_ns: u64,
    pub request_arrival_ns: u64,
    pub absolute_deadline_ns: Option<u64>,
    pub application_priority: f64,
    pub memory_mb: usize,
    pub io_bytes: u64,
    pub sched_period_ns: Option<u64>,
    pub registry_dir: String,
    pub result_path: String,
    #[serde(default)]
    pub admission_delayed_ns: u64,
    #[serde(default = "worker_default_true")]
    pub start_stopped: bool,
}

fn worker_default_true() -> bool { true }
