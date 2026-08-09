#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/results/smoke-proposed}"
[[ ${EUID:-$(id -u)} -eq 0 ]] || { echo "run with sudo" >&2; exit 1; }
"$ROOT/scripts/run_experiment.sh" proposed "$ROOT/configs/smoke.toml" "$OUT" 0
python3 "$ROOT/analysis/analyze_results.py" "$OUT" \
    --out "$OUT/run-summary.csv" --aggregate-out "$OUT/aggregate-summary.csv"
