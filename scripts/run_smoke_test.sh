#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/results/smoke-proposed}"

if [[ ${EUID:-$(id -u)} -eq 0 ]]; then
    echo "error: run this smoke test as the regular user, not with sudo" >&2
    exit 2
fi

"$ROOT/scripts/run_experiment.sh" \
    proposed \
    "$ROOT/configs/smoke.toml" \
    "$OUT" \
    0

python3 "$ROOT/analysis/analyze_results.py" \
    "$OUT" \
    --out "$OUT/run-summary.csv" \
    --aggregate-out "$OUT/aggregate-summary.csv"
