#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_ROOT=${1:-$ROOT/results/periodic-matrix}
REPETITIONS=${REPETITIONS:-10}
METHODS=${METHODS:-"eevdf rustland proposed sched-deadline"}
ORDER_SEED=${ORDER_SEED:-204729}
mkdir -p "$OUT_ROOT"
printf 'repetition\torder\n' > "$OUT_ROOT/execution-order.tsv"
read -r -a METHOD_ARRAY <<< "$METHODS"
for rep in $(seq 0 $((REPETITIONS - 1))); do
    mapfile -t ORDER < <(
        python3 - "$((ORDER_SEED + rep))" "${METHOD_ARRAY[@]}" <<'PY'
import random
import sys
items = sys.argv[2:]
random.Random(int(sys.argv[1])).shuffle(items)
print("\n".join(items))
PY
    )
    printf '%s\t%s\n' "$rep" "${ORDER[*]}" >> "$OUT_ROOT/execution-order.tsv"
    for method in "${ORDER[@]}"; do
        out="$OUT_ROOT/$method/rep-$(printf '%02d' "$rep")"
        echo "== periodic / $method / repetition $rep =="
        "$ROOT/scripts/run_experiment.sh" "$method" "$ROOT/configs/periodic.toml" "$out" "$rep"
        "$ROOT/scripts/collect_system_info.sh" "$out/environment-after.txt" >/dev/null
    done
done
