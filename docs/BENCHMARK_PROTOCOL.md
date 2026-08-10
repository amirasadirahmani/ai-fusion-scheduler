# Benchmark Protocol - AI Fusion Scheduler

This document defines the benchmark protocol for paper-oriented evaluation of AI Fusion Scheduler. The goal is to freeze methodology before final results are generated.

## 1. Objective

Evaluate whether application-aware sched_ext scheduling and workload-scoped pressure-aware admission improve soft-deadline execution for mixed AI/edge workloads compared with standard Linux scheduling and stock RustLand scheduling.

## 2. Compared methods

Minimum methods:

```text
eevdf                    Linux default EEVDF baseline
rustland                 stock scx_rustland baseline
proposed-no-admission    AFS scheduling without admission control
proposed                 full AFS with admission control
```

Optional ablations:

```text
proposed-no-aging
proposed-no-priority
proposed-no-application-deadline
proposed-virtual-deadline
```

SCHED_DEADLINE should only be evaluated on methodologically comparable periodic/sporadic workloads.

## 3. Workload profiles

Final profiles must be sustained enough for meaningful PSI `avg10` behavior.

Intended profiles:

- **Light**: capacity is sufficient; admission should rarely reject.
- **Saturated**: load is near capacity; scheduling differences should become visible.
- **Overload**: load exceeds capacity; admission should trade acceptance for deadline goodput.

Current configs are count-driven. If `duration_s` is not enforced by the generator, final configs must state that actual duration is measured from timestamps.

## 4. CPU isolation

Each run partitions CPUs into:

```text
control CPUs  = generator + userspace scheduler + orchestration
workload CPUs = application workers
```

For a 6-CPU VM and `experiment.cpus = 4`:

```text
control CPUs  = 0,1
workload CPUs = 2,3,4,5
```

All compared methods must use the same CPU partition.

## 5. Workload-scoped PSI

Admission must use workload cgroup CPU pressure:

```text
<delegated-run-scope>/afs-workload/cpu.pressure
```

It must not use system-wide `/proc/pressure/cpu` for admission decisions.

System-wide PSI may be recorded as environment/provenance data only.

## 6. Shared manifests

For each configuration and repetition:

1. Generate or select one task manifest.
2. Reuse that same manifest across all compared methods.
3. Record manifest SHA256.
4. Validate every run against the manifest.

This avoids comparing different random workload traces.

## 7. Run order

For each configuration and repetition, scheduler method order should be randomized deterministically by a seed. The execution order must be stored in `execution-order.tsv`.

## 8. Quiescence

Before each run:

- verify `sched_ext` is disabled,
- wait for CPU PSI quiescence,
- record initial system PSI,
- start scheduler only after quiescence.

This reduces carryover pressure between runs.

## 9. Validation

Every run must produce:

```text
summary.json
tasks.csv
run-input.json
run-provenance.txt
run-validation.json
```

`run-validation.json` must be `status = ok` before a run is considered valid.

## 10. Primary metrics

Do not use only admitted-task deadline miss rate. Admission can reduce miss count by rejecting tasks.

Primary paper metrics:

```text
offered_deadline_tasks
deadline_successes
deadline_goodput = deadline_successes / offered_deadline_tasks
accepted_deadline_tasks
accepted_deadline_misses
accepted_miss_rate = accepted_deadline_misses / accepted_deadline_tasks
rejection_rate
completion_rate
```

Secondary metrics:

```text
P50/P95 response time
throughput
CPU utilization
Jain fairness
scheduler CPU overhead
workload PSI
local/user/kernel dispatch statistics
failed/cancelled/bounced dispatches
```

## 11. Reporting rules

- Always report offered-load metrics and admitted-load metrics together.
- Report per-class results for critical, interactive, and batch work.
- Do not label diagnostic/pilot results as final benchmark results.
- Preserve the exact Git commit, tag, kernel, scx revision, and configs used.
- Do not change workload definitions after seeing final results.

## 12. Experimental freeze criteria

Before final matrix execution:

- code is committed and tagged,
- configs are committed and tagged,
- scx revision is recorded,
- kernel version is recorded,
- run scripts are committed,
- analysis scripts are committed,
- paper metrics are frozen,
- pilot runs pass validation.
