# Final primary benchmark snapshot

This directory is the tracked, curated snapshot of the final primary benchmark
analysis produced from `results/paper-final-v1`.

The raw `results/` tree is intentionally ignored by Git because it contains the
full benchmark output. The files here preserve the publication-facing summary,
integrity report, per-run analysis, aggregate analysis, and hashes of the raw
run artifacts.

## Provenance

- Experimental freeze tag: `experimental-freeze-v1`
- Freeze commit: `edcd8eb347f998fc68c705ca770eaa645c8e78be`
- Kernel: `7.1.3+deb13-arm64`
- scx revision: `7a58a3a1a857fa748ea5a3bda5ad312afbbeca5b`
- Primary matrix: 3 workloads x 4 methods x 10 repetitions = 120 runs
- Validation: 120/120 `status = ok`
- Shared manifests: 30/30 verified
- Run-to-manifest hashes: 120/120 verified
- Kernel OOM events during the final matrix window: none

## Tracked files

- `FINAL_RESULTS_SUMMARY.md`: publication-facing primary results table
- `FINAL_INTEGRITY.md`: final matrix integrity report
- `aggregate-summary.csv`: 12 workload/method aggregate rows
- `run-summary.csv`: 120 per-run analysis rows
- `raw-artifact-sha256.txt`: hashes for the raw result artifacts

These are the frozen primary-matrix results. Remaining ablations and secondary
evaluation items are recorded separately and must not overwrite this snapshot.
