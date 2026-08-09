# Baseline protocols

## Common controls

All methods must use the same kernel, toolchain, hardware, CPU governor, task
manifest, random seed, affinity policy, warm-up and result parser. Record the
exact command line for every run.

## EEVDF

- no sched_ext scheduler loaded;
- workers use `SCHED_OTHER`;
- default nice value unless the experiment explicitly studies nice/cgroup;
- verify `/sys/kernel/sched_ext/state` is `disabled` before launch.

## SCHED_DEADLINE

Use only `configs/periodic.toml` or another periodic/sporadic scenario with a
defensible `runtime <= deadline <= period` contract. The runner applies
`sched_setattr()` while each worker is stopped. If kernel admission rejects a
reservation, record the failure rather than silently relaxing parameters.

## Standard scx_rustland

- unmodified binary from the frozen `scx` release/commit;
- partial mode only;
- no project metadata consumed by the baseline;
- preserve its stdout/stderr and available stats.

## Proposed scheduler

- partial mode;
- application metadata read from the PID registry;
- application deadline + remaining runtime + priority + aging;
- admission control executed by the workload runner before process release;
- scheduler counters written to `scheduler-stats.json`.

## Ablations

- no aging: disable it in both config/runner and scheduler;
- no admission: runner admits all otherwise valid tasks;
- virtual deadline: scheduler ignores `absolute_deadline_ns` and uses a
  rustland-like behavior-derived virtual deadline.
