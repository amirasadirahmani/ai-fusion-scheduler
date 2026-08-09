#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONFIG=${1:-$ROOT/configs/saturated.toml}
OUT_ROOT=${2:-$ROOT/results/ablations}
REPETITION=${3:-0}

AFS_SCHEDULER_ARGS="" AFS_GENERATOR_ARGS="" \
    "$ROOT/scripts/run_experiment.sh" proposed "$CONFIG" "$OUT_ROOT/full" "$REPETITION"
AFS_SCHEDULER_ARGS="--no-aging" AFS_GENERATOR_ARGS="--no-aging" \
    "$ROOT/scripts/run_experiment.sh" proposed "$CONFIG" "$OUT_ROOT/no-aging" "$REPETITION"
AFS_SCHEDULER_ARGS="" AFS_GENERATOR_ARGS="--no-admission" \
    "$ROOT/scripts/run_experiment.sh" proposed "$CONFIG" "$OUT_ROOT/no-admission" "$REPETITION"
AFS_SCHEDULER_ARGS="--no-application-deadline" AFS_GENERATOR_ARGS="--no-application-deadline" \
    "$ROOT/scripts/run_experiment.sh" proposed "$CONFIG" "$OUT_ROOT/virtual-deadline" "$REPETITION"
