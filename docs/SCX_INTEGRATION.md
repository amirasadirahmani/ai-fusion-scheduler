# scx integration notes

## Default pin

The adapter declares `scx_rustland_core = 2.4.13`, corresponding to the July 2026 published line. The official project may require a different compatible version for Linux 6.12.95. Freeze one working version and record it in `ENVIRONMENT.md`.

## Build model

`scx_rustland_core::RustLandBuilder` generates the BPF skeleton during the scheduler crate build. The scheduler includes the generated `bpf.rs`, receives `QueuedTask` records, selects a task in Rust, sends a `DispatchedTask`, then calls `notify_complete()`.

## Partial mode

The adapter requests partial switching so only benchmark tasks with policy `SCHED_EXT` enter the custom scheduler. The workload generator applies policy 7 (`SCHED_EXT`) after each worker registers and stops itself.

## Compatibility adjustment checklist

If compilation fails after pinning a different `scx` release, compare these items with the release's `scx_rlfifo` example:

- `BpfScheduler::init` argument list;
- generated `QueuedTask` and `DispatchedTask` fields;
- `select_cpu` signature;
- stats accessor names;
- `notify_complete` return type;
- `shutdown_and_report` and restart handling;
- `RustLandBuilder` build method.

Do not patch policy logic until the unmodified FIFO/Round-Robin example from the same release builds and runs.
