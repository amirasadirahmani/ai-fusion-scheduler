# Implementation and Go/No-Go plan

## Stage 0 - scope confirmation

Confirm with the supervisor that the required contribution is single-node CPU
scheduling. GPU scheduling, Kubernetes placement, multi-node offloading and a
new fusion algorithm are explicitly out of scope.

## Stage 1 - host freeze

1. Boot Linux 6.12.95.
2. Select one compatible `scx` tag/commit.
3. Record the kernel config hash, toolchain, CPU governor and hardware in
   `ENVIRONMENT.md`.
4. Generate and retain `Cargo.lock` with `scripts/freeze_dependencies.sh`.

## Stage 2 - week-one smoke test

- run upstream `scx_simple`;
- verify `/sys/kernel/sched_ext/state` becomes `enabled`;
- terminate it and verify the state becomes `disabled`;
- retain kernel logs.

## Stage 3 - week-two rustland test

- run unmodified `scx_rustland` in partial mode;
- move only a stopped benchmark worker to `SCHED_EXT`;
- collect user/kernel dispatch counters;
- unload safely at least ten times.

**Go condition:** both upstream schedulers run and unload repeatedly without a
verifier error, runnable-task stall or loss of SSH access.

## Stage 4 - user-space policy

Run policy unit tests and the deterministic simulator before loading the custom
scheduler. Verify lower laxity wins, aging improves bounded waiting, negative
laxity is handled, and ties are deterministic.

## Stage 5 - custom adapter

Build `afs-rustland-scheduler`, start it in partial mode, and run
`configs/smoke.toml`. Do not proceed to final experiments until the complete
load -> dispatch -> collect -> unload cycle is repeatable.

## Stage 6 - experiments

Generate one task manifest per scenario and reuse it for all schedulers. Run
main baselines first, then selected ablations. Randomize method order and use at
least ten repetitions for final claims.
