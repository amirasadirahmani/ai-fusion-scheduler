# Result schema

Each real-process run directory contains:

- `task-manifest.json` or `fusion-task-manifest.json`: deterministic workload;
- `run-input.json`: command inputs, CPU count and effective policy flags;
- `runtime-estimates.json`: final EWMA estimates by workload/stage key;
- `tasks/`: worker specification and result JSON files;
- `tasks.csv`: task-level canonical data;
- `summary.json`: basic run counts and run timestamps;
- `scheduler.log`: proposed scheduler log, where applicable;
- `scheduler-stats.json`: final `scx_rustland_core`/adapter counters for the proposed method.

## Task-level columns

| Field | Meaning |
|---|---|
| `admission` | `admit`, `delay`, or `reject` |
| `delayed_ns` | accumulated non-blocking admission delay |
| `release_time_ns` | conceptual request/stage eligibility time (`CLOCK_MONOTONIC`) |
| `absolute_deadline_ns` | application completion deadline used for evaluation |
| `estimated_runtime_ns` | calibrated/EWMA CPU-time estimate at launch |
| `actual_cpu_time_ns` | process CPU time consumed by a completed worker |
| `response_time_ns` | completion minus request arrival |
| `deadline_missed` | completion later than application deadline |
| `workflow_id`, `stage_id` | fusion-pipeline identity |

Deadline miss ratio is computed only among admitted, completed tasks with a
reported application deadline. Rejection rate and deadline goodput are always
reported beside it.

## Scheduler-overhead fields

`analysis/analyze_results.py` merges `scheduler-stats.json` into run summaries.
Important fields include:

- scheduler CPU time and scheduler CPU fraction;
- metadata cache hits, registry reads and misses;
- local dispatches, attempts and dequeued tasks;
- `notify_complete` cycles and average tasks per cycle;
- decision-only wall time per cycle and per dispatch;
- full cycle time, which may include waiting inside `notify_complete`;
- rustland-core user/kernel/cancelled/bounced/failed dispatch counters;
- scheduler congestion events.

Decision-only time is the meaningful hot-path wall-time estimate. Full cycle
wall time is retained for observability but may include blocking/wakeup time.
