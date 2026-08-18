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

## 13. Pilot qualification and bounded memory working sets

Pilot runs are used to validate the experimental methodology and harness,
not to generate final publication measurements.

The first sustained overload pilot exposed a host-level memory exhaustion
confound. Synthetic workers retain their configured memory allocation during
the memory phase, so large per-worker working sets combined with bursty
concurrency could trigger the global Linux OOM killer independently of the
scheduler under test.

Before experimental freeze, sustained workload memory working sets were
bounded as follows:

- mixed interactive task: 8 MiB per worker
- memory-oriented batch task: 16 MiB per worker

This correction did not change task counts, release patterns, burst sizes,
runtimes, deadlines, priorities, scheduler parameters, or admission
parameters.

The corrected overload workload was validated across all four primary
methods without a new OOM event.

Paper pilot v2 then completed all 36 planned qualification runs with
structural validation success.

Pilot v1 measurements from before the memory correction are diagnostic only.
Pilot v2 performance values are qualification evidence rather than final
paper results.

Scheduler or workload parameters must not be retuned from pilot v2
performance outcomes before final evaluation.

## 14. Frozen paper metric definitions and aggregation

The following definitions are frozen before the final benchmark matrix.

### Primary run-level metrics

For every run:

- `offered_deadline_tasks` is the number of offered tasks with an
  `absolute_deadline_ns`, including tasks rejected by admission control.
- `deadline_successes` is the number of offered deadline-bearing tasks that
  complete successfully and on time.
- `deadline_goodput = deadline_successes / offered_deadline_tasks`.
  Rejected deadline-bearing tasks remain in the denominator and are not
  successes.
- `accepted_deadline_tasks` is the number of deadline-bearing tasks not
  rejected by admission control.
- `accepted_deadline_misses = accepted_deadline_tasks - deadline_successes`.
  An accepted task that is late, fails, or does not complete successfully is
  not credited as a deadline success.
- `accepted_miss_rate =
  accepted_deadline_misses / accepted_deadline_tasks`.
- `rejection_rate = rejected_tasks / total_tasks`.
- `completion_rate = completed_tasks / total_tasks`.

`deadline_goodput` is the principal deadline metric because it measures useful
deadline service against the offered workload and cannot be improved merely by
rejecting difficult tasks. `accepted_miss_rate` is reported alongside it to
show scheduling quality after admission.

The same offered-load, admitted-load, completion, response-time, and deadline
metrics are computed separately for `critical`, `interactive`, and `batch`
tasks.

### Secondary run-level metrics

- P50 and P95 response time are computed over successfully completed tasks.
- Throughput is successful completions divided by measured run duration.
- Workload CPU utilization is workload CPU time normalized by configured
  workload CPUs and measured run duration.
- Jain class-service fairness is secondary and does not replace the
  offered-load deadline metrics.
- Workload CPU PSI is read from
  `workload-cpu-pressure-final.txt`.
- `workload_psi_some_avg10_final` is the final `some avg10` snapshot.
- `workload_psi_some_total_us` is the cumulative cgroup-v2 `some total`.
- `workload_psi_some_stall_fraction =
  workload_psi_some_total_us / (duration_s * 1e6)`.
- AFS internal overhead is reported from `scheduler-stats.json` as scheduler
  CPU time/fraction and decision statistics when available. These counters
  are not used as a direct cross-method overhead comparison with EEVDF or
  stock RustLand.

Analysis uses the requested method in `run-provenance.txt` as the experimental
method identity and retains the underlying scheduler as `scheduler_impl`.
Therefore `proposed` and `proposed-no-admission` remain separate methods.

### Repetition-level aggregation

The final primary matrix uses ten repetitions per workload/method combination.

- A run/repetition is the unit of aggregation. Tasks from different
  repetitions are not pooled as independent samples.
- Metrics are computed per run first and then aggregated across repetitions.
- Aggregates are reported as the arithmetic mean plus a two-sided 95%
  Student-t confidence-interval half-width:
  `t_(n-1,0.975) * s / sqrt(n)`.
- For final `n = 10`, the implementation uses `df = 9` and `t = 2.262`.
- Structurally inapplicable blank metrics are omitted from that metric's
  aggregation rather than converted to zero.
- Shared manifests preserve pairing across methods for the same
  workload/repetition. Any cross-method delta must use repetition-matched
  run-level differences, not pooled task samples.
- Per-class metrics are also computed per run before aggregation.

Pilot and diagnostic runs are excluded from final-paper aggregation and are
not used for post-hoc workload or policy tuning.

After this metric freeze, presentation code may change, but the raw metric
definitions, repetition unit, and aggregation rules above must not change in
response to final benchmark outcomes.
