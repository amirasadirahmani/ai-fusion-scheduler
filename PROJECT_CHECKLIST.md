# Project completion checklist

## Environment and safety

- [ ] Linux 6.12.95 confirmed
- [ ] exact `scx` Tag/Commit recorded
- [ ] kernel config hash recorded
- [ ] Rust/Clang/libbpf/bpftool versions recorded
- [ ] second SSH/root session available
- [ ] emergency stop tested
- [ ] upstream `scx_simple` and `scx_rustland` smoke tests passed

## Build and correctness

- [ ] `cargo fmt --all -- --check`
- [ ] user-space unit tests pass
- [ ] proposed adapter builds against frozen `scx` API
- [ ] partial-mode scheduler loads and unloads safely
- [ ] deterministic manifests match across methods
- [ ] worker start-stop handshake validated
- [ ] deadline, aging and admission ablations validated

## Experiments

- [ ] EEVDF runs complete
- [ ] periodic/sporadic `SCHED_DEADLINE` runs complete
- [ ] unmodified `scx_rustland` runs complete
- [ ] proposed method runs complete
- [ ] selected ablations complete
- [x] fusion case study complete
- [ ] at least 10 final repetitions per required method/scenario
- [ ] method order randomized
- [ ] thermal/governor/background-service conditions recorded

## Analysis and reporting

- [ ] admission/rejection/goodput shown together
- [ ] DMR, mean/P95 response, throughput and fairness reported
- [ ] scheduler CPU and dispatch counters reported
- [ ] confidence intervals generated
- [ ] limitations and unsuccessful cases retained
- [ ] raw results, configs, manifests and environment snapshot archived

<!-- CHECKPOINT-2026-08-09:START -->
## Checkpoint 2026-08-09

### Environment / kernel

- [x] sched_ext-capable kernel installed
- [x] BPF/BTF prerequisites verified
- [x] passwordless sudo validated for lab VM

### scx / RustLand

- [x] scx commit identified
- [x] stock RustLand built
- [x] attach/detach validated
- [x] stress test completed

### AFS scheduler

- [x] dependency compatibility fixed
- [x] rustland-scheduler current API integration fixed
- [x] kernel 7.1 BPF helper usage fixed
- [x] struct_ops attach EINVAL root cause found
- [x] scheduler renamed to `afs_rustland`
- [x] AFS attach/detach validated
- [x] metadata registry validated
- [x] real userspace dispatch observed

### Admission

- [x] Admission v1 false rejection reproduced
- [x] Admission OFF diagnostic completed
- [x] Admission v2 implemented
- [x] score-aware interference helper added
- [x] generator updated
- [x] simulator updated
- [x] policy-core tests pass 10/10
- [x] Admission v2 real Smoke passes
- [ ] stress validation under heavier overload
- [ ] calibration across pressure levels

### Reproducibility freeze

- [ ] apply documentation checkpoint
- [ ] capture environment snapshot
- [ ] inspect `git status`
- [ ] inspect `git diff`
- [ ] record Cargo.lock hash
- [ ] record release binary hashes
- [ ] preserve Smoke diagnostic directories
- [ ] create Git checkpoint commit/tag

### Benchmark engineering

- [ ] inventory configs
- [ ] inventory scripts
- [ ] inspect runners
- [ ] define final output naming/layout
- [ ] freeze manifests
- [ ] implement baseline lifecycle checks
- [ ] implement 10-repetition runner
- [ ] implement integrity validation

### Baselines

- [ ] EEVDF
- [ ] stock RustLand
- [ ] Proposed AFS
- [ ] SCHED_DEADLINE where fair

### Ablations

- [ ] no-aging
- [ ] no-admission
- [ ] no-application-deadline

### Workloads

- [ ] CPU contention
- [ ] mixed CPU/memory/I/O
- [ ] deadline pressure sweep
- [ ] class mix sweep
- [ ] arrival pattern sweep
- [ ] Massive Data Fusion case study

### Analysis

- [ ] deadline miss ratio
- [ ] mean/P95 latency
- [ ] throughput
- [ ] CPU utilization
- [ ] Jain fairness
- [ ] scheduler overhead
- [ ] dispatch/congestion/admission metrics
- [ ] aggregate 10 repetitions
- [ ] plots/tables
- [ ] variability/confidence reporting

### Academic outputs

- [ ] update proposal
- [ ] update report/thesis
- [ ] update paper draft
- [ ] update slides
- [ ] update README/diagrams/methodology/limitations

### Professor demo

- [ ] create `scripts/demo_for_professor.sh`
- [ ] representative real run
- [ ] aggregate benchmark result display
- [ ] Persian spoken script
- [ ] clean screen recording
<!-- CHECKPOINT-2026-08-09:END -->

## Paper-oriented experimental checkpoint — 2026-08-10

### Completed

- [x] sched_ext-capable kernel installed and validated
- [x] pinned scx revision
- [x] stock RustLand baseline builds and attaches
- [x] proposed Rust AFS scheduler attaches and detaches
- [x] real workload/control-plane CPU partitioning
- [x] deterministic shared manifests
- [x] corrected benchmark release clock
- [x] workload-scoped cgroup-v2 CPU PSI
- [x] automatic run validation
- [x] benchmark provenance
- [x] admission-v3 engineering diagnostics
- [x] light scoped-PSI sanity run
- [x] overload admission/no-admission diagnostic A/B
- [x] paper benchmark protocol draft
- [x] paper plan draft

### Before experimental freeze

- [x] freeze sustained light workload
- [x] freeze sustained saturated workload
- [x] freeze sustained overload workload
- [x] ensure traces are long enough for meaningful PSI avg10 behavior
- [x] run three-repetition pilot matrix
- [x] inspect scheduler overhead measurements
- [x] finalize paper metrics and aggregation
- [x] freeze experiment configuration

Pilot qualification note:

- [x] identify host-level OOM confound in pilot v1
- [x] bound sustained-profile memory working sets before freeze
- [x] revalidate overload with all four primary methods
- [x] complete paper-pilot-v2 with 36/36 valid runs
- [x] keep pilot results separate from final publication measurements

### Final evaluation

- [x] EEVDF baseline
- [x] stock RustLand baseline
- [x] AFS without admission
- [x] full AFS
- [x] no-aging ablation
- [x] no-application-deadline ablation
- [x] periodic/sporadic SCHED_DEADLINE comparison
- [x] fusion case study
- [x] ten repetitions of the final benchmark matrix
- [x] statistical aggregation and plots

### Publication and presentation

- [x] final results section
- [ ] final proposal refresh
- [ ] final report
- [ ] final PowerPoint results slides
- [ ] professor demonstration recording
- [ ] conference-paper draft
