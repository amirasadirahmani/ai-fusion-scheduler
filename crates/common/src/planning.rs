use crate::{ExperimentConfig, WorkloadClass, WorkloadKind};
use anyhow::{Context, Result};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Exp};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedTask {
    pub task_id: u64,
    pub name: String,
    pub workload_class: WorkloadClass,
    pub kind: WorkloadKind,
    pub release_offset_ns: u64,
    pub estimated_runtime_ns: u64,
    pub actual_runtime_ns: u64,
    pub relative_deadline_ns: Option<u64>,
    pub application_priority: f64,
    pub memory_mb: usize,
    pub io_bytes: u64,
    pub workflow_id: u64,
    pub stage_id: u32,
    pub sched_period_ns: Option<u64>,
    #[serde(default)]
    pub depends_on_task_id: Option<u64>,
}

impl PlannedTask {
    pub fn write_manifest(tasks: &[Self], path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_vec_pretty(tasks)?;
        fs::write(path, data).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn read_manifest(path: impl AsRef<Path>) -> Result<Vec<Self>> {
        let path = path.as_ref();
        let data = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_slice(&data)
            .with_context(|| format!("failed to parse {}", path.display()))
    }
}

pub fn generate_task_manifest(cfg: &ExperimentConfig, repetition: u32) -> Result<Vec<PlannedTask>> {
    let seed = cfg
        .experiment
        .seed
        .wrapping_add(u64::from(repetition).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut tasks = Vec::new();
    let mut task_id = 1u64;

    for template in &cfg.workloads {
        let mut offset_ms = template.start_delay_ms as f64;
        let exp = if template.arrival == "poisson" {
            let mean = template.interarrival_ms.max(1) as f64;
            Some(Exp::new(1.0 / mean)?)
        } else {
            None
        };

        for index in 0..template.count {
            if index > 0 {
                offset_ms += match template.arrival.as_str() {
                    "fixed" => template.interarrival_ms as f64,
                    "poisson" => exp.as_ref().unwrap().sample(&mut rng),
                    "bursty" => {
                        let burst = template.burst_size.max(1);
                        if index % burst == 0 {
                            template.interarrival_ms as f64
                        } else {
                            0.5
                        }
                    }
                    _ => template.interarrival_ms as f64,
                };
            }

            let jitter = template.runtime_jitter_pct / 100.0;
            let multiplier = if jitter > 0.0 {
                rng.gen_range((1.0 - jitter)..=(1.0 + jitter))
            } else {
                1.0
            };
            let actual_runtime_ms = (template.runtime_ms as f64 * multiplier).round().max(1.0);
            tasks.push(PlannedTask {
                task_id,
                name: format!("{}-{}", template.name, index + 1),
                workload_class: template.class,
                kind: template.kind,
                release_offset_ns: (offset_ms * 1_000_000.0).round().max(0.0) as u64,
                estimated_runtime_ns: template.runtime_ms.saturating_mul(1_000_000),
                actual_runtime_ns: (actual_runtime_ms * 1_000_000.0) as u64,
                relative_deadline_ns: template.deadline_ms.map(|v| v.saturating_mul(1_000_000)),
                application_priority: template.priority,
                memory_mb: template.memory_mb,
                io_bytes: template.io_bytes,
                workflow_id: template.workflow_id,
                stage_id: template.stage_id,
                sched_period_ns: template.sched_period_ms.map(|v| v.saturating_mul(1_000_000)),
                depends_on_task_id: None,
            });
            task_id = task_id.saturating_add(1);
        }
    }

    tasks.sort_by_key(|t| (t.release_offset_ns, t.task_id));
    Ok(tasks)
}

/// Generate a deterministic dependency-aware manifest for the lightweight
/// massive-data-fusion case study. Each workflow is a chain of stages.
pub fn generate_pipeline_manifest(cfg: &ExperimentConfig, repetition: u32) -> Result<Vec<PlannedTask>> {
    let pipeline = cfg.pipeline.as_ref().context("configuration has no pipeline section")?;
    let mut tasks = Vec::new();
    let mut task_id = 1u64;
    let seed = cfg
        .experiment
        .seed
        .wrapping_add(u64::from(repetition).wrapping_mul(0xD1B5_4A32_D192_ED03));
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    for workflow_index in 0..pipeline.workflows {
        let workflow_id = u64::from(workflow_index) + 1;
        let release_offset_ns = u64::from(workflow_index)
            .saturating_mul(pipeline.workflow_interarrival_ms)
            .saturating_mul(1_000_000);
        let mut previous = None;

        for (stage_index, stage) in pipeline.stages.iter().enumerate() {
            // Small deterministic jitter prevents every workflow from being identical while
            // preserving reproducibility across scheduler baselines.
            let multiplier = rng.gen_range(0.92_f64..=1.08_f64);
            let actual_runtime_ns = ((stage.runtime_ms as f64 * multiplier).round().max(1.0)
                * 1_000_000.0) as u64;
            tasks.push(PlannedTask {
                task_id,
                name: format!("wf-{workflow_id}-{}", stage.name),
                workload_class: stage.class,
                kind: stage.kind,
                release_offset_ns,
                estimated_runtime_ns: stage.runtime_ms.saturating_mul(1_000_000),
                actual_runtime_ns,
                relative_deadline_ns: stage.deadline_ms.map(|v| v.saturating_mul(1_000_000)),
                application_priority: stage.priority,
                memory_mb: stage.memory_mb,
                io_bytes: stage.io_bytes,
                workflow_id,
                stage_id: stage_index as u32 + 1,
                sched_period_ns: None,
                depends_on_task_id: previous,
            });
            previous = Some(task_id);
            task_id = task_id.saturating_add(1);
        }
    }

    tasks.sort_by_key(|t| (t.release_offset_ns, t.workflow_id, t.stage_id, t.task_id));
    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AdmissionConfig, ExperimentSection, PolicyConfig, SchedulerMethod, WorkloadTemplate};

    #[test]
    fn deterministic_manifest() {
        let cfg = ExperimentConfig {
            experiment: ExperimentSection {
                name: "x".into(),
                seed: 1,
                scheduler: SchedulerMethod::Simulator,
                cpus: 1,
                repetitions: 1,
                duration_s: 1,
                warmup_s: 0,
                registry_dir: "/tmp/x".into(),
                output_dir: "results".into(),
                max_concurrent: 10,
            },
            policy: PolicyConfig::default(),
            admission: AdmissionConfig::default(),
            workloads: vec![WorkloadTemplate {
                name: "w".into(),
                class: WorkloadClass::Interactive,
                kind: WorkloadKind::Cpu,
                count: 3,
                arrival: "poisson".into(),
                start_delay_ms: 0,
                interarrival_ms: 10,
                burst_size: 0,
                runtime_ms: 10,
                runtime_jitter_pct: 10.0,
                deadline_ms: Some(100),
                priority: 0.5,
                memory_mb: 1,
                io_bytes: 1,
                workflow_id: 0,
                stage_id: 0,
                sched_period_ms: None,
            }],
            pipeline: None,
        };
        let a = generate_task_manifest(&cfg, 0).unwrap();
        let b = generate_task_manifest(&cfg, 0).unwrap();
        assert_eq!(a[1].release_offset_ns, b[1].release_offset_ns);
        assert_eq!(a[1].actual_runtime_ns, b[1].actual_runtime_ns);
    }
}
