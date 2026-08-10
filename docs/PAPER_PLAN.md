# Paper Plan - AI Fusion Scheduler

## Working title

Application-Aware Soft-Deadline Scheduling with Workload-Scoped Pressure Admission on Linux sched_ext

## Core idea

AFS combines user-space `sched_ext` scheduling with application metadata and workload-scoped pressure-aware admission control. The key methodological correction is to isolate workload CPU pressure from sched_ext userspace control-plane pressure using cgroup-v2 `cpu.pressure`.

## Candidate contributions

1. A sched_ext-based application-aware scheduler for mixed critical, interactive, and batch AI workloads.
2. A soft-deadline scoring model using deadline laxity, priority, and aging.
3. A workload-scoped PSI admission controller that avoids system-wide pressure contamination.
4. A reproducible benchmark harness with CPU partitioning, shared manifests, validation, and provenance.
5. An empirical evaluation separating scheduler-policy effects from admission/load-shedding effects.

## Key baselines

- Linux EEVDF.
- Stock scx_rustland.
- AFS without admission.
- Full AFS with admission.

## Main evaluation question

Does AFS increase deadline goodput under saturation and overload while preserving reasonable acceptance and completion rates?

## Secondary questions

- How much improvement comes from the scheduling policy alone?
- How much comes from admission control?
- Does workload-scoped PSI avoid false pressure signals from userspace sched_ext control-plane activity?
- Which workload classes benefit or lose under overload?
- What is the scheduler overhead?

## Current pilot evidence

Current evidence is diagnostic and qualification-only.

An early overload pilot exposed a host-level OOM confound caused by large
per-worker synthetic memory working sets under bursty concurrency. The
working sets were bounded before experimental freeze without changing task
counts, release patterns, runtimes, deadlines, priorities, scheduler
parameters, or admission parameters.

After correction, a four-method overload diagnostic completed without a new
OOM event. Paper pilot v2 subsequently completed 36/36 structurally valid
runs across light, saturated, and overload workloads, four primary methods,
and three repetitions.

The light profile behaves as a low-pressure floor regime, while saturated
and overload profiles expose differentiated behavior among the compared
methods.

These pilot performance values are not final claims and must not be used to
retune workload or scheduler parameters.

Final claims will be based only on the post-freeze benchmark matrix and its
predefined offered-load and admitted-load metrics.

## Risks

- Novelty must be checked against sched_ext, Linux scheduling, edge-cloud scheduling, deadline scheduling, and overload control literature.
- Admission can make miss rate look better by rejecting tasks, so deadline goodput relative to offered load must be primary.
- Current finite traces are count-driven; final trace duration must be explicitly documented.
- VM timing and UTM environment effects must be reported as limitations.

## Paper structure

1. Introduction
2. Background: Linux sched_ext, soft deadlines, PSI, cgroup v2
3. Problem statement
4. AFS design
5. Admission control and workload-scoped PSI
6. Benchmark harness and methodology
7. Evaluation
8. Discussion and limitations
9. Related work
10. Conclusion

## Frozen analysis policy

Before the final matrix, paper metrics and aggregation are frozen. The primary
deadline outcome is offered deadline goodput, always reported with rejection,
completion, and accepted miss rate. Results are also reported for critical,
interactive, and batch classes.

Each run is analyzed independently and the final ten repetitions are
aggregated at the repetition level with a two-sided 95% Student-t confidence
interval. Tasks are not pooled across repetitions. Workload-cgroup CPU PSI and
AFS-internal scheduler overhead are secondary metrics.

The three-repetition pilot remains qualification evidence only; it is excluded
from final-paper aggregates and is not used for post-hoc tuning.
