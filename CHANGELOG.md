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
