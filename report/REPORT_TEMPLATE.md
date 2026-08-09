# Final project report template

## 1. Introduction and research question

State the distinction between application completion deadlines, EEVDF virtual
deadlines, SCHED_DEADLINE reservations, and scx_rustland behavior-derived
virtual deadlines.

## 2. Related work

Cover EEVDF, EDF+CBS/SCHED_DEADLINE, sched_ext, scx_rustland,
scx_rusty and scx_lavd/bpfland. Do not claim EDF, laxity or aging as new.

## 3. System model and architecture

Describe metadata, admission, Rust policy, rustland_core/BPF bridge and partial
mode. Include the single-node Cloud/Edge and data-fusion boundary.

## 4. Policy

Present runtime estimation, laxity, normalized priority, aging, tie-breaking,
time slices and ADMIT/DELAY/REJECT rules.

## 5. Implementation

Record the exact kernel release, scx commit, Cargo.lock hash, toolchain and
hardware. Describe the Go/No-Go tests and safety procedure.

## 6. Methodology

Document task manifests, workloads, baseline parameterization, repetitions,
warm-up, randomization, metrics and statistical treatment.

## 7. Results

Report deadline miss ratio, rejection rate and goodput together. Include P95,
throughput, fairness, CPU utilization and user/kernel dispatch overhead.

## 8. Ablation and sensitivity

Compare no-aging, no-admission and virtual-deadline variants. Add runtime
estimate error sensitivity if schedule permits.

## 9. Massive data fusion case study

Explain ingestion, preprocessing, feature extraction, fusion, alert and
archival stages. The contribution is predictable execution, not a new fusion
algorithm.

## 10. Limitations and future work

Discuss single-node scope, partial mode, runtime-estimation uncertainty,
virtualization/thermal effects, BPF hot-path migration and GPU/cluster work.
