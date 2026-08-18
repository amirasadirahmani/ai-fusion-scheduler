# Experimental Freeze v1

This file records the paper benchmark freeze immediately before the final
measurement matrix.

- Tag: `experimental-freeze-v1`
- Branch: `benchmark-engineering`
- Pre-freeze base commit: `fe4eb5e67e40288a158fc6e17bf7ded227a31393`
- Freeze recorded: `2026-08-10T23:54:32+03:30`
- Kernel: `7.1.3+deb13-arm64`
- scx revision: `7a58a3a1a857fa748ea5a3bda5ad312afbbeca5b`
- sched_ext state during freeze check: `disabled`

## Frozen scope

The Git tag `experimental-freeze-v1` is the authoritative snapshot for the final experiment
configuration, workload definitions, scheduler/admission policy, benchmark
harness, validation logic, paper metric definitions, and aggregation rules.

The primary final matrix is:

- 3 sustained workloads: light, saturated, overload,
- 4 primary methods: EEVDF, stock RustLand, AFS without admission, full AFS,
- 10 repetitions per workload/method,
- 120 primary final runs total.

The shared input manifest for each workload/repetition is reused across methods
to preserve pairing.

## Qualification evidence

`results/paper-pilot-v2` contains the qualification pilot:

- 36/36 run validations are `status = ok`,
- 36 runs analyze into 12 workload/method groups,
- each group contains 3 repetitions,
- the four primary methods remain distinct in aggregation,
- offered/admitted deadline metrics, per-class metrics, response metrics,
  workload-scoped CPU PSI, and AFS-internal overhead instrumentation are
  available.

Pilot measurements are qualification-only. They are excluded from final-paper
aggregation and are not tuning inputs.

## Frozen analysis contract

The principal deadline metric is offered deadline goodput. Rejection,
completion, and accepted miss rate are reported alongside it. Per-class
critical/interactive/batch results are retained.

Each repetition is the statistical unit. Tasks are not pooled across
repetitions. Final aggregate means use two-sided 95% Student-t confidence
intervals as defined in `docs/BENCHMARK_PROTOCOL.md`.

AFS scheduler CPU/decision counters are reported as AFS-internal overhead
measurements, not as a direct cross-method overhead comparison with EEVDF or
stock RustLand.

After this tag, workload definitions, scheduler/admission policy, metric
definitions, or aggregation rules must not be changed in response to final
benchmark outcomes. Any necessary post-freeze correction must be explicitly
documented as a new freeze/version.
