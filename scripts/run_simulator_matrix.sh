#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${AFS_BIN_DIR:-$ROOT/target/release}/afs-policy-simulator"
[[ -x "$BIN" ]] || { echo "error: $BIN missing; run scripts/build_userspace.sh" >&2; exit 1; }
OUT=${1:-$ROOT/results/simulator}
mkdir -p "$OUT"
for cfg in smoke light saturated overload; do
    "$BIN" --config "$ROOT/configs/$cfg.toml" --output "$OUT/${cfg}-full.csv"
    "$BIN" --config "$ROOT/configs/$cfg.toml" --output "$OUT/${cfg}-no-aging.csv" --no-aging
    "$BIN" --config "$ROOT/configs/$cfg.toml" --output "$OUT/${cfg}-no-admission.csv" --no-admission
    "$BIN" --config "$ROOT/configs/$cfg.toml" --output "$OUT/${cfg}-virtual-deadline.csv" --virtual-deadline
done
