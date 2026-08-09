# Proposal-to-implementation traceability

| Proposal item | Implementation |
|---|---|
| Config-driven Rust workload generator | `crates/workload-generator`, `configs/*.toml` |
| Application/task/pipeline metadata | `crates/common/src/model.rs`, `crates/metadata-manager` |
| Deadline, remaining runtime, priority and aging | `crates/policy-core/src/scoring.rs` |
| Calibration/EWMA runtime estimate | `crates/policy-core/src/runtime.rs`, generator updates |
| `ADMIT/DELAY/REJECT` with CPU PSI | `crates/policy-core/src/admission.rs`, `psi.rs` |
| Rust user-space policy over `sched_ext` | `crates/rustland-scheduler` |
| EEVDF baseline | `scripts/run_experiment.sh eevdf ...` |
| `SCHED_DEADLINE` baseline | periodic config and `common/src/linux.rs` |
| unmodified `scx_rustland` baseline | `scripts/lib.sh`, `run_experiment.sh rustland ...` |
| aging/admission/application-deadline ablations | `scripts/run_ablations.sh` |
| Massive Data Fusion case study | `crates/fusion-pipeline`, `configs/fusion-pipeline.toml` |
| DMR, P95, throughput, fairness and overhead | `analysis/analyze_results.py`, `plot_results.py` |
| reproducibility and environment freeze | `ENVIRONMENT.md`, `scripts/collect_system_info.sh`, `freeze_dependencies.sh` |
| one-semester execution plan | `docs/IMPLEMENTATION_PLAN.md`, `PROJECT_CHECKLIST.md` |

The final Persian proposal used as the implementation basis is included as
`docs/final-proposal-fa.pdf`.
