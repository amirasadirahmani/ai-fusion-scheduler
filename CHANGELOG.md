# Changelog

## 0.2.0

Added final deliverables requested for course submission:

- MacBook M1 Pro execution guide
- Installation and run guide
- Dependency Lock document
- Pinned Python requirements file
- Linux/macOS environment YAML files
- Evaluation dataset documentation
- Synthetic sample results and generated plots for analysis pipeline testing
- Final report template in Persian
- Full Persian PowerPoint slide deck
- Package contents table
- Updated validation status

## 0.1.0

Initial project package with Rust workspace, configs, scripts, simulator, workload generator, fusion pipeline and sched_ext integration skeleton.

<!-- CHECKPOINT-2026-08-09:START -->
## 2026-08-09 — sched_ext end-to-end checkpoint / Admission v2

### Added

- Real sched_ext validation on Debian ARM64 kernel `7.1.3+deb13-arm64`
- shared score-aware `relevant_interference_ns(...)`
- Admission regression tests
- consistent Admission semantics in real generator and simulator
- reproducibility checkpoint workflow

### Changed

- Rust/toolchain/dependency compatibility pins
- common Linux helper exports
- RustLand integration for current `BpfScheduler::init(...)`
- scheduler ops name: `afs-rustland` → `afs_rustland`
- BPF slice/DSQ writes migrated to current sched_ext helpers
- Admission model: `(active + own) / cpus` → `own + relevant_active / cpus`, then safety factor

### Fixed

- missing sched_ext in initial Debian kernel
- RustLand scheduler compile/API mismatches
- struct_ops attach `EINVAL`
- root-owned registry permission error
- demonstrated Admission v1 False Rejection in Smoke workload

### Validation

```text
afs-policy-core: 10 passed; 0 failed
```

Real Admission v2 Smoke:

```text
tasks=7
completed=7
rejected=0
failed=0
deadline_misses=0
```

`critical-cpu-2`:

```text
response ~= 31.54 ms
deadline = 120 ms
deadline_missed = false
```

These are Development/Smoke validation results, not final thesis/paper benchmark results.
<!-- CHECKPOINT-2026-08-09:END -->

## Paper benchmark harness checkpoint (2026-08-10)

- Added enforced workload/control-plane CPU partitioning.
- Added workload CPU affinity.
- Hardened sched_ext scheduler lifecycle and cleanup.
- Added pre-run CPU-pressure quiescence checking.
- Added shared manifests for identical scheduler comparisons.
- Added deterministic randomized benchmark execution ordering.
- Added automatic structural run validation.
- Extended provenance with kernel, source revisions, CPU partition and binary
  hashes.
- Corrected workload release-clock placement so initialization I/O does not
  consume application deadline budget.
- Added workload-scoped cgroup-v2 CPU PSI.
- Removed system-wide CPU PSI from application admission decisions.
- Added admission deadline-slack handling and relevant-interference
  accounting.
- Validated the revised harness using smoke, light, idle-PSI, and overload
  diagnostic experiments.

Diagnostic and pilot measurements at this checkpoint are not final benchmark
or publication results.

## Paper pilot qualification and workload memory correction (2026-08-10)

- The first sustained overload pilot exposed a host-level OOM confound rather
  than a scheduler failure.
- Synthetic workers retain their configured memory working set during the
  memory phase. Large per-worker allocations combined with bursty concurrency
  could therefore exhaust the 8 GiB experimental VM.
- Mixed interactive working sets were bounded to 8 MiB and explicit
  memory-oriented batch working sets to 16 MiB across the sustained paper
  profiles.
- Task counts, release patterns, burst structure, runtimes, deadlines,
  priorities, scheduler parameters, and admission parameters were unchanged.
- The corrected overload profile was revalidated with EEVDF, stock RustLand,
  AFS without admission, and full AFS without a new OOM event.
- Paper pilot v2 completed 36/36 structurally valid runs across three
  workloads, four methods, and three repetitions.
- Pilot v1 is diagnostic only and must not be mixed with final benchmark
  measurements.
- Pilot v2 performance values are qualification evidence only and are not
  final publication claims or tuning inputs.

## Paper metric freeze preparation (2026-08-10)

- separated requested method identity from the underlying scheduler
  implementation so `proposed` and `proposed-no-admission` aggregate
  independently,
- added offered deadline goodput, accepted miss rate, completion/rejection,
  P50/P95 response time, per-class metrics, and workload-cgroup CPU PSI,
- added aggregate regression coverage and two-sided Student-t 95% confidence
  intervals,
- froze repetition-level aggregation and prohibited pooling tasks across
  repetitions,
- kept pilot measurements qualification-only and outside final-paper
  aggregation.

## Final primary matrix completed (2026-08-11)

- completed the frozen 3-workload x 4-method x 10-repetition primary matrix,
- validated 120/120 runs with no validation failures,
- verified 30 shared manifests and all 120 run-to-manifest hashes,
- confirmed no kernel OOM events during the final matrix window,
- produced final per-run and aggregate analysis with ten repetitions per
  workload/method group,
- preserved a tracked publication-facing snapshot under `docs/final-results/`,
- moved post-results documentation and remaining evaluation work to the
  `paper-finalization` branch while preserving `experimental-freeze-v1`.
