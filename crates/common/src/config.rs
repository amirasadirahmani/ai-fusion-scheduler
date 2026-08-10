use crate::{SchedulerMethod, WorkloadClass, WorkloadKind};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    pub experiment: ExperimentSection,
    #[serde(default)]
    pub policy: PolicyConfig,
    #[serde(default)]
    pub admission: AdmissionConfig,
    #[serde(default)]
    pub workloads: Vec<WorkloadTemplate>,
    #[serde(default)]
    pub pipeline: Option<PipelineConfig>,
}

impl ExperimentConfig {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let cfg: Self =
            toml::from_str(&data).with_context(|| format!("failed to parse {}", path.display()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        if self.experiment.name.trim().is_empty() {
            bail!("experiment.name cannot be empty");
        }
        if self.experiment.cpus == 0 {
            bail!("experiment.cpus must be positive");
        }
        if self.experiment.repetitions == 0 {
            bail!("experiment.repetitions must be positive");
        }
        if self.experiment.max_concurrent == 0 {
            bail!("experiment.max_concurrent must be positive");
        }
        if self.policy.quantum_ms == 0 {
            bail!("policy.quantum_ms must be positive");
        }
        if self.policy.aging_horizon_ms == 0 {
            bail!("policy.aging_horizon_ms must be positive");
        }
        if self.policy.critical_slice_ms == 0
            || self.policy.interactive_slice_ms == 0
            || self.policy.batch_slice_ms == 0
        {
            bail!("all policy time slices must be positive");
        }
        if !self.policy.alpha_ms.is_finite()
            || !self.policy.beta_ms.is_finite()
            || self.policy.alpha_ms < 0.0
            || self.policy.beta_ms < 0.0
        {
            bail!("policy alpha_ms and beta_ms must be finite and non-negative");
        }
        match self
            .policy
            .runtime_source
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "queued" | "proc" => {}
            other => bail!("unknown policy.runtime_source {other}; expected queued or proc"),
        }
        if !self.policy.runtime_ewma_alpha.is_finite()
            || self.policy.runtime_ewma_alpha <= 0.0
            || self.policy.runtime_ewma_alpha > 1.0
        {
            bail!("policy.runtime_ewma_alpha must be in (0,1]");
        }
        if !self.admission.psi_delay_threshold.is_finite()
            || !self.admission.psi_reject_threshold.is_finite()
            || self.admission.psi_delay_threshold < 0.0
            || self.admission.psi_reject_threshold < 0.0
            || self.admission.psi_delay_threshold > self.admission.psi_reject_threshold
        {
            bail!("invalid admission PSI thresholds");
        }
        if !self.admission.safety_factor.is_finite() || self.admission.safety_factor < 1.0 {
            bail!("admission.safety_factor must be finite and at least 1.0");
        }
        if !self.admission.reject_below_priority.is_finite()
            || !(0.0..=1.0).contains(&self.admission.reject_below_priority)
        {
            bail!("admission.reject_below_priority must be in [0,1]");
        }
        if self.admission.enabled
            && self.admission.max_delay_ms > 0
            && self.admission.delay_step_ms == 0
        {
            bail!("admission.delay_step_ms must be positive when delay is enabled");
        }
        for w in &self.workloads {
            w.validate()?;
        }
        if self.workloads.is_empty() && self.pipeline.is_none() {
            bail!("configuration needs workloads or a pipeline");
        }
        if let Some(pipeline) = &self.pipeline {
            pipeline.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSection {
    pub name: String,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default)]
    pub scheduler: SchedulerMethod,
    #[serde(default = "default_cpus")]
    pub cpus: usize,
    #[serde(default = "default_repetitions")]
    pub repetitions: u32,
    #[serde(default = "default_duration_s")]
    pub duration_s: u64,
    #[serde(default = "default_warmup_s")]
    pub warmup_s: u64,
    #[serde(default = "default_registry_dir")]
    pub registry_dir: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    #[serde(default = "default_alpha_ms")]
    pub alpha_ms: f64,
    #[serde(default = "default_beta_ms")]
    pub beta_ms: f64,
    #[serde(default = "default_aging_horizon_ms")]
    pub aging_horizon_ms: u64,
    #[serde(default = "default_batch_base_laxity_ms")]
    pub batch_base_laxity_ms: i64,
    #[serde(default = "default_quantum_ms")]
    pub quantum_ms: u64,
    #[serde(default = "default_critical_slice_ms")]
    pub critical_slice_ms: u64,
    #[serde(default = "default_interactive_slice_ms")]
    pub interactive_slice_ms: u64,
    #[serde(default = "default_batch_slice_ms")]
    pub batch_slice_ms: u64,
    #[serde(default = "default_true")]
    pub application_deadline: bool,
    #[serde(default = "default_true")]
    pub aging: bool,
    #[serde(default = "default_true")]
    pub priority: bool,
    #[serde(default = "default_runtime_source")]
    pub runtime_source: String,
    #[serde(default = "default_runtime_ewma_alpha")]
    pub runtime_ewma_alpha: f64,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            alpha_ms: default_alpha_ms(),
            beta_ms: default_beta_ms(),
            aging_horizon_ms: default_aging_horizon_ms(),
            batch_base_laxity_ms: default_batch_base_laxity_ms(),
            quantum_ms: default_quantum_ms(),
            critical_slice_ms: default_critical_slice_ms(),
            interactive_slice_ms: default_interactive_slice_ms(),
            batch_slice_ms: default_batch_slice_ms(),
            application_deadline: true,
            aging: true,
            priority: true,
            runtime_source: default_runtime_source(),
            runtime_ewma_alpha: default_runtime_ewma_alpha(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_psi_delay")]
    pub psi_delay_threshold: f64,
    #[serde(default = "default_psi_reject")]
    pub psi_reject_threshold: f64,
    #[serde(default = "default_safety_factor")]
    pub safety_factor: f64,
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
    #[serde(default = "default_delay_step_ms")]
    pub delay_step_ms: u64,
    #[serde(default = "default_min_reject_priority")]
    pub reject_below_priority: f64,
}

impl Default for AdmissionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            psi_delay_threshold: default_psi_delay(),
            psi_reject_threshold: default_psi_reject(),
            safety_factor: default_safety_factor(),
            max_delay_ms: default_max_delay_ms(),
            delay_step_ms: default_delay_step_ms(),
            reject_below_priority: default_min_reject_priority(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadTemplate {
    pub name: String,
    #[serde(default)]
    pub class: WorkloadClass,
    #[serde(default)]
    pub kind: WorkloadKind,
    #[serde(default = "default_count")]
    pub count: u32,
    #[serde(default = "default_arrival")]
    pub arrival: String,
    #[serde(default)]
    pub start_delay_ms: u64,
    #[serde(default = "default_interarrival_ms")]
    pub interarrival_ms: u64,
    #[serde(default)]
    pub burst_size: u32,
    #[serde(default = "default_runtime_ms")]
    pub runtime_ms: u64,
    #[serde(default)]
    pub runtime_jitter_pct: f64,
    #[serde(default)]
    pub deadline_ms: Option<u64>,
    #[serde(default = "default_priority")]
    pub priority: f64,
    #[serde(default = "default_memory_mb")]
    pub memory_mb: usize,
    #[serde(default = "default_io_bytes")]
    pub io_bytes: u64,
    #[serde(default)]
    pub workflow_id: u64,
    #[serde(default)]
    pub stage_id: u32,
    #[serde(default)]
    pub sched_period_ms: Option<u64>,
}

impl WorkloadTemplate {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("workload name cannot be empty");
        }
        if self.count == 0 {
            bail!("workload {} count must be positive", self.name);
        }
        if self.runtime_ms == 0 {
            bail!("workload {} runtime_ms must be positive", self.name);
        }
        if !self.priority.is_finite() || !(0.0..=1.0).contains(&self.priority) {
            bail!(
                "workload {} priority must be finite and in [0, 1]",
                self.name
            );
        }
        if !self.runtime_jitter_pct.is_finite()
            || self.runtime_jitter_pct < 0.0
            || self.runtime_jitter_pct > 100.0
        {
            bail!(
                "workload {} runtime_jitter_pct must be finite and in [0, 100]",
                self.name
            );
        }
        if let Some(deadline) = self.deadline_ms {
            if deadline == 0 {
                bail!("workload {} deadline must be positive", self.name);
            }
            if let Some(period) = self.sched_period_ms {
                if period == 0 || deadline > period {
                    bail!(
                        "workload {} expects 0 < deadline_ms <= sched_period_ms",
                        self.name
                    );
                }
            }
        } else if self.sched_period_ms.is_some() {
            bail!(
                "workload {} sched_period_ms requires deadline_ms",
                self.name
            );
        }
        match self.arrival.as_str() {
            "fixed" | "poisson" | "bursty" => {}
            other => bail!("workload {} unknown arrival pattern {other}", self.name),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    #[serde(default = "default_workflows")]
    pub workflows: u32,
    #[serde(default = "default_interarrival_ms")]
    pub workflow_interarrival_ms: u64,
    pub stages: Vec<PipelineStage>,
}

impl PipelineConfig {
    fn validate(&self) -> Result<()> {
        if self.workflows == 0 {
            bail!("pipeline.workflows must be positive");
        }
        if self.stages.is_empty() {
            bail!("pipeline.stages cannot be empty");
        }
        for (idx, stage) in self.stages.iter().enumerate() {
            if stage.name.trim().is_empty() {
                bail!("pipeline stage {idx} name cannot be empty");
            }
            if stage.runtime_ms == 0 {
                bail!("pipeline stage {idx} runtime_ms must be positive");
            }
            if !stage.priority.is_finite() || !(0.0..=1.0).contains(&stage.priority) {
                bail!("pipeline stage {idx} priority must be finite and in [0,1]");
            }
            if stage.deadline_ms == Some(0) {
                bail!("pipeline stage {idx} deadline_ms must be positive when present");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    pub name: String,
    #[serde(default)]
    pub class: WorkloadClass,
    #[serde(default)]
    pub kind: WorkloadKind,
    pub runtime_ms: u64,
    #[serde(default)]
    pub deadline_ms: Option<u64>,
    #[serde(default = "default_priority")]
    pub priority: f64,
    #[serde(default = "default_memory_mb")]
    pub memory_mb: usize,
    #[serde(default = "default_io_bytes")]
    pub io_bytes: u64,
}

fn default_seed() -> u64 {
    104_729
}
fn default_cpus() -> usize {
    2
}
fn default_repetitions() -> u32 {
    1
}
fn default_duration_s() -> u64 {
    60
}
fn default_warmup_s() -> u64 {
    5
}
fn default_registry_dir() -> String {
    "/tmp/ai-fusion-scheduler/registry".into()
}
fn default_output_dir() -> String {
    "results".into()
}
fn default_max_concurrent() -> usize {
    64
}
fn default_alpha_ms() -> f64 {
    100.0
}
fn default_beta_ms() -> f64 {
    500.0
}
fn default_aging_horizon_ms() -> u64 {
    2_000
}
fn default_batch_base_laxity_ms() -> i64 {
    5_000
}
fn default_quantum_ms() -> u64 {
    2
}
fn default_critical_slice_ms() -> u64 {
    2
}
fn default_interactive_slice_ms() -> u64 {
    4
}
fn default_batch_slice_ms() -> u64 {
    8
}
fn default_runtime_source() -> String {
    "queued".into()
}
fn default_runtime_ewma_alpha() -> f64 {
    0.25
}
fn default_true() -> bool {
    true
}
fn default_psi_delay() -> f64 {
    20.0
}
fn default_psi_reject() -> f64 {
    60.0
}
fn default_safety_factor() -> f64 {
    1.20
}
fn default_max_delay_ms() -> u64 {
    500
}
fn default_delay_step_ms() -> u64 {
    25
}
fn default_min_reject_priority() -> f64 {
    0.8
}
fn default_count() -> u32 {
    1
}
fn default_arrival() -> String {
    "fixed".into()
}
fn default_interarrival_ms() -> u64 {
    50
}
fn default_runtime_ms() -> u64 {
    100
}
fn default_priority() -> f64 {
    0.5
}
fn default_memory_mb() -> usize {
    64
}
fn default_io_bytes() -> u64 {
    8 * 1024 * 1024
}
fn default_workflows() -> u32 {
    10
}
