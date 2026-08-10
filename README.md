# AI Fusion Scheduler

A research prototype for the one-semester proposal:

> **Design and Evaluation of a Rust-Based Application-Aware Soft-Deadline Scheduler for Artificial Intelligence Workloads Using Linux `sched_ext`**

The repository implements the user-space parts of the project and includes a separately built adapter for `scx_rustland_core`. The core policy combines:

- application-declared absolute deadlines;
- estimated remaining CPU time;
- normalized application priority;
- explicit aging for bounded waiting;
- `ADMIT / DELAY / REJECT` admission control using CPU PSI;
- workload and fusion-pipeline metadata (`workflow_id`, `stage_id`).

It also includes a deterministic policy simulator, a config-driven workload generator, a lightweight massive-data-fusion pipeline, experiment scripts, baseline orchestration, CSV metrics, and analysis scripts.

Persian quick start: [`docs/QUICK_START_FA.md`](docs/QUICK_START_FA.md).
Proposal traceability: [`docs/SOURCE_TRACEABILITY.md`](docs/SOURCE_TRACEABILITY.md).

## Important validation status

The pure user-space code is designed to build on stable Rust 1.82 or newer. The `sched_ext` adapter is pinned to the published `scx_rustland_core` API, excluded from the default workspace build, and compiled separately. It **must be compiled and smoke-tested on the target Linux 6.12.95 host**, because this build environment does not provide a Rust toolchain, root access, BTF, or a `sched_ext` kernel. See [`docs/VALIDATION_STATUS.md`](docs/VALIDATION_STATUS.md).

## Repository map

```text
crates/common/              Shared config, task, result and Linux helpers
crates/metadata-manager/    PID-keyed metadata registry
crates/policy-core/         Laxity, aging, scoring, runtime estimation, admission
crates/policy-simulator/    Discrete-time deterministic simulator
crates/workload-worker/     CPU, memory, I/O and mixed worker process
crates/workload-generator/  Config-driven real-process experiment runner
crates/fusion-pipeline/     Lightweight multi-stage data-fusion case study
crates/rustland-scheduler/  scx_rustland_core adapter (separate target-host build)
configs/                    Smoke, light, saturated, overload and fusion scenarios
scripts/                    Environment, build, baseline, matrix and safety scripts
analysis/                   CSV aggregation and plotting
```

## 1. Prepare the target host

Use a dedicated test machine or VM snapshot first. The proposal fixes the experimental kernel to Linux 6.12.95 and requires a compatible pinned `scx` release or commit.

```bash
./scripts/check_environment.sh
./scripts/collect_system_info.sh results/environment.txt
```

Expected kernel options include `CONFIG_SCHED_CLASS_EXT=y`, BPF, BPF JIT and BTF. The official `sched_ext` API has no cross-version stability guarantee, so record the exact kernel and `scx` commit.

## 2. Install dependencies on Ubuntu/Debian

Review the script before running it:

```bash
./scripts/install_deps_ubuntu.sh --print
```

Then install manually or run the printed commands. A Rust toolchain >= 1.82, Clang >= 16, libbpf, bpftool, libelf, zlib, zstd and pkg-config are required for the scheduler adapter.

## 3. Build and test user-space components

```bash
./scripts/build_userspace.sh
cargo test --workspace --exclude afs-rustland-scheduler
```

Run the deterministic simulator first:

```bash
cargo run -p afs-policy-simulator -- \
  --config configs/saturated.toml \
  --output results/sim-saturated.csv
```

## 4. Build the sched_ext adapter

The adapter is intentionally excluded from the default workspace build.

```bash
./scripts/build_sched_ext.sh
```

The default dependency pin is documented in `crates/rustland-scheduler/Cargo.toml`. If the target host uses a different compatible `scx` commit, update the pin once, record it in `ENVIRONMENT.md`, and do not change it during experiments.

## 5. Safe smoke test

The proposed scheduler uses **partial mode**: only tasks explicitly assigned to `SCHED_EXT` are handled by it. This reduces the risk of scheduling the whole login session or SSH daemon.

Open a second root shell or SSH session before the test.

```bash
sudo ./scripts/run_smoke_test.sh
```

Emergency stop:

```bash
sudo ./scripts/emergency_stop.sh
```

The kernel also provides the `SysRq-S` sched_ext abort mechanism when configured.

## 6. Run a single real-process experiment

EEVDF baseline:

```bash
./scripts/run_experiment.sh eevdf configs/light.toml results/eevdf-light
```

Standard `scx_rustland` baseline:

```bash
sudo ./scripts/run_experiment.sh rustland configs/light.toml results/rustland-light
```

Proposed scheduler:

```bash
sudo ./scripts/run_experiment.sh proposed configs/light.toml results/proposed-light
```

`SCHED_DEADLINE` is only valid for the periodic/sporadic configuration:

```bash
sudo ./scripts/run_experiment.sh sched-deadline configs/periodic.toml results/sched-deadline-periodic
```

## 7. Run the fusion case study

```bash
sudo ./scripts/run_fusion_case.sh proposed configs/fusion-pipeline.toml results/fusion-proposed
```

The case study models ingestion, preprocessing, feature extraction, fusion, decision/alert and archival stages. It evaluates the scheduler; it does not claim a new fusion algorithm.

## 8. Analyze results

```bash
python3 analysis/analyze_results.py results --out results/summary.csv
python3 analysis/plot_results.py results/summary.csv --out-dir results/plots
```

Primary metrics:

- deadline miss ratio among admitted tasks;
- goodput (completed within deadline per second);
- admission, delay and rejection rates;
- mean and P95 response time;
- throughput;
- Jain's fairness index;
- scheduler user-space CPU time and rustland dispatch counters.

## Experiment discipline

- keep kernel, `scx` commit, CPU governor and toolchain fixed;
- use the same seed and generated task manifest for all schedulers;
- randomize baseline execution order;
- use warm-up and cool-down intervals;
- record thermal throttling and background services;
- run at least 10 repetitions for final results;
- do not interpret a lower miss ratio as success if it was achieved by rejecting most tasks.

See [`docs/EXPERIMENTS.md`](docs/EXPERIMENTS.md) and [`docs/SAFETY.md`](docs/SAFETY.md).

## License

GPL-2.0-only. The `scx_rustland_core` backend is also GPL-2.0-only.

## تکمیل های نسخه 0.2.0

این نسخه علاوه بر کد اصلی پروژه، موارد تحویلی زیر را نیز شامل می شود:

- راهنمای اجرای MacBook M1 Pro: `docs/MACBOOK_M1_PRO_GUIDE_FA.md`
- راهنمای نصب و اجرا: `docs/INSTALL_AND_RUN_FA.md`
- Dependency Lock و محیط بازتولید: `docs/DEPENDENCY_LOCK.md`
- مجموعه ارزیابی: `docs/EVALUATION_DATASET_FA.md`
- نتایج نمونه ساختگی برای تست تحلیل: `results/sample`
- قالب گزارش نهایی: `report/FINAL_REPORT_FA.md`
- اسلاید ارائه کامل: `presentation/ai_fusion_scheduler_presentation.pptx`

توجه: نتایج `results/sample` واقعی نیستند و فقط برای بررسی ابزار تحلیل و نمودارها استفاده می شوند. نتایج نهایی پژوهش باید روی سیستم Linux 6.12.95 تولید و در `results/real` ذخیره شوند.

## Paper-oriented benchmark checkpoint — 2026-08-10

The project has reached the first paper-oriented benchmark-engineering
checkpoint on branch `benchmark-engineering`.

Checkpoint:

- Git commit: `f7001bfd20c4007cdc9f082bffb26e4d360081e9`
- Git tag: `checkpoint-paper-harness-v1`
- Linux kernel: `7.1.3+deb13-arm64`
- sched_ext enabled kernel
- pinned scx revision:
  `7a58a3a1a857fa748ea5a3bda5ad312afbbeca5b`

The experimental harness now provides:

- real CPU partitioning between application workload and control plane;
- deterministic shared task manifests across compared schedulers;
- corrected workload release-clock semantics;
- workload-scoped cgroup-v2 CPU PSI for admission decisions;
- pre-run quiescence checks;
- deterministic sched_ext attach/detach lifecycle;
- run provenance and binary/source revision recording;
- automatic structural result validation.

The current light and overload measurements are diagnostic/pilot evidence
only and are not final publication results.

Paper-oriented documentation:

- `docs/BENCHMARK_PROTOCOL.md`
- `docs/PAPER_PLAN.md`
- `docs/PROPOSAL_METHODOLOGY_CHECKPOINT_FA.md`
- `presentation/SLIDE_UPDATE_NOTES.md`

The next experimental phase is to freeze sustained light, saturated, and
overload workload definitions and perform repeated pilot evaluation before
the final benchmark matrix.
