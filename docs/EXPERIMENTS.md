# Experiment plan

## Main methods

1. **EEVDF** - default fair scheduler on the fixed kernel.
2. **SCHED_DEADLINE** - only for periodic/sporadic workloads with defensible `runtime/deadline/period` parameters.
3. **scx_rustland** - unmodified standard scheduler at the pinned release/commit.
4. **Proposed** - application deadline, remaining runtime, priority, aging and admission control.

## Required ablations

- proposed without aging;
- proposed without admission control;
- proposed without application deadlines, using a rustland-like virtual deadline.

Ablations run only on selected saturated and bursty scenarios.

## Workload intensities

- smoke: correctness, one or two tasks;
- light: below capacity, overhead focus;
- saturated: near capacity, policy comparison;
- overload/bursty: admission and tail-latency focus;
- periodic: fair comparison with `SCHED_DEADLINE`;
- fusion: multi-stage case study.

## Reproducibility

Each run writes:

- the exact input TOML;
- generated task manifest;
- environment snapshot;
- raw task CSV/JSONL;
- scheduler stdout/stderr;
- kernel log tail;
- scheduler metrics snapshots;
- a run manifest with seed and timestamps.

Use at least 10 repetitions for the final report. Randomize method order to reduce time-of-day and thermal bias.

## Metrics

- admission, delay and rejection rates;
- completion rate;
- deadline miss ratio among admitted/completed tasks;
- goodput;
- mean, median, P95 and P99 response time;
- throughput;
- per-class CPU service and Jain fairness;
- scheduler process CPU time;
- user, kernel, cancelled, bounced and failed dispatches;
- congestion events;
- mean tasks per `notify_complete` cycle.

## Fairness warning

A policy that rejects most work can trivially reduce its miss ratio. The report must present miss ratio, rejection rate and goodput side by side.
