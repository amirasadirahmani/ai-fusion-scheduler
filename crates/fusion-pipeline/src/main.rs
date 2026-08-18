use afs_common::{generate_pipeline_manifest, ExperimentConfig, PlannedTask, SchedulerMethod};
use anyhow::{bail, Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

#[derive(Debug, Parser)]
#[command(about = "Lightweight dependency-aware massive-data-fusion case study")]
struct Args {
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, default_value = "proposed")]
    method: String,
    #[arg(long, default_value_t = 0)]
    repetition: u32,
    #[arg(long)]
    generator: Option<PathBuf>,
    #[arg(long)]
    worker: Option<PathBuf>,
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();
    let args = Args::parse();
    let cfg = ExperimentConfig::from_path(&args.config)?;
    if cfg.pipeline.is_none() {
        bail!(
            "{} does not contain a [pipeline] section",
            args.config.display()
        );
    }
    let method = SchedulerMethod::from_str(&args.method).map_err(anyhow::Error::msg)?;
    fs::create_dir_all(&args.output_dir)?;
    let manifest = generate_pipeline_manifest(&cfg, args.repetition)?;
    let manifest_path = args.output_dir.join("fusion-task-manifest.json");
    PlannedTask::write_manifest(&manifest, &manifest_path)?;
    write_pipeline_description(&args.output_dir, &cfg, &manifest)?;

    let generator = args
        .generator
        .unwrap_or(resolve_sibling_binary("afs-workload-generator")?);
    if !generator.exists() && !args.dry_run {
        bail!(
            "workload generator not found at {}; build user-space binaries or pass --generator",
            generator.display()
        );
    }

    let mut command = Command::new(generator);
    command
        .arg("--config")
        .arg(&args.config)
        .arg("--output-dir")
        .arg(&args.output_dir)
        .arg("--method")
        .arg(method.to_string())
        .arg("--repetition")
        .arg(args.repetition.to_string())
        .arg("--manifest")
        .arg(&manifest_path);
    if let Some(worker) = args.worker {
        command.arg("--worker").arg(worker);
    }
    if args.dry_run {
        command.arg("--dry-run");
    }

    let status = command
        .status()
        .context("failed to execute workload generator")?;
    if !status.success() {
        bail!("workload generator failed with {status}");
    }
    Ok(())
}

fn write_pipeline_description(
    output_dir: &Path,
    cfg: &ExperimentConfig,
    manifest: &[PlannedTask],
) -> Result<()> {
    let pipeline = cfg.pipeline.as_ref().expect("validated above");
    let doc = serde_json::json!({
        "experiment": &cfg.experiment.name,
        "workflows": pipeline.workflows,
        "workflow_interarrival_ms": pipeline.workflow_interarrival_ms,
        "stages": &pipeline.stages,
        "task_count": manifest.len(),
        "dependency_model": "Each stage depends on the previous stage of the same workflow. Stage deadlines are relative to stage eligibility.",
        "scope": "Scheduler evaluation only; no claim of a new data-fusion algorithm."
    });
    fs::write(
        output_dir.join("fusion-pipeline.json"),
        serde_json::to_vec_pretty(&doc)?,
    )?;
    Ok(())
}

fn resolve_sibling_binary(name: &str) -> Result<PathBuf> {
    let current = std::env::current_exe()?;
    let parent = current
        .parent()
        .context("current executable has no parent")?;
    Ok(parent.join(name))
}
