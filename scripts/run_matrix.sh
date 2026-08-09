#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_ROOT=${1:-$ROOT/results/matrix}
REPETITIONS=${REPETITIONS:-10}
METHODS=${METHODS:-"eevdf rustland proposed"}
CONFIGS=${CONFIGS:-"light saturated overload"}
ORDER_SEED=${ORDER_SEED:-104729}
mkdir -p "$OUT_ROOT"
printf 'repetition\tconfig\torder\n' > "$OUT_ROOT/execution-order.tsv"
read -r -a METHOD_ARRAY <<< "$METHODS"
read -r -a CONFIG_ARRAY <<< "$CONFIGS"
for rep in $(seq 0 $((REPETITIONS - 1))); do
    for cfg_index in "${!CONFIG_ARRAY[@]}"; do
        cfg=${CONFIG_ARRAY[$cfg_index]}
        seed=$((ORDER_SEED + rep * 1000 + cfg_index))
        mapfile -t ORDER < <(
            python3 - "$seed" "${METHOD_ARRAY[@]}" <<'PY'
import random
import sys
seed = int(sys.argv[1])
items = sys.argv[2:]
random.Random(seed).shuffle(items)
print("\n".join(items))
PY
        )
        printf '%s\t%s\t%s\n' "$rep" "$cfg" "${ORDER[*]}" >> "$OUT_ROOT/execution-order.tsv"
        for method in "${ORDER[@]}"; do
            out="$OUT_ROOT/$cfg/$method/rep-$(printf '%02d' "$rep")"
            echo "== $cfg / $method / repetition $rep =="
            "$ROOT/scripts/run_experiment.sh" "$method" "$ROOT/configs/$cfg.toml" "$out" "$rep"
            "$ROOT/scripts/collect_system_info.sh" "$out/environment-after.txt" >/dev/null
        done
    done
done
