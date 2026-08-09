# Architecture

```text
TOML experiment configuration
          |
          v
Config-driven workload or fusion runner
          |
          +--> Admission controller (runtime estimate + CPU PSI)
          |
          +--> Worker process writes PID metadata and stops itself
          |          |
          |          v
          |    PID-keyed metadata registry
          |          |
          +--> Runner assigns EEVDF / SCHED_EXT / SCHED_DEADLINE
                     and resumes worker
                                |
                                v
              Application-aware Rust scheduler
            deadline + remaining runtime + priority + aging
                                |
                                v
                    scx_rustland_core bridge
                                |
                                v
                   BPF backend + Linux sched_ext
```

## Metadata contract

All time values use `CLOCK_MONOTONIC` nanoseconds. Runtime means CPU time unless a workload explicitly declares wall-clock I/O duration.

- `task_id`: stable task identifier;
- `process_id`: Linux PID;
- `workflow_id`: fusion or AI workflow;
- `stage_id`: workflow stage;
- `workload_class`: critical, interactive or batch;
- `absolute_deadline_ns`: optional absolute completion deadline;
- `estimated_total_runtime_ns`;
- `estimated_remaining_runtime_ns`;
- `application_priority`: normalized [0, 1];
- `release_time_ns`;
- `kind`: CPU, memory, I/O or mixed.

## Policy

For a deadline task:

```text
laxity = absolute_deadline - now - estimated_remaining_runtime
score  = laxity - alpha * normalized_priority - beta * normalized_aging
```

Lower score is dispatched first. For tasks without an application deadline, a configurable batch base laxity is used. Aging is bounded by an aging horizon, ensuring waiting tasks eventually become competitive.

## Admission

The admission controller estimates completion pressure from active estimated work per CPU, the new task's estimated runtime, a safety factor, and CPU PSI.

- `ADMIT`: spawn and run now;
- `DELAY`: postpone until the delay budget expires or pressure falls;
- `REJECT`: do not spawn; record a rejected result.

Miss ratio is always reported together with rejection rate and goodput.
