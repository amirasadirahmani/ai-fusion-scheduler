#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source "$ROOT/scripts/lib.sh"

OUT_ROOT=${1:-$ROOT/results/matrix}
REPETITIONS=${REPETITIONS:-10}
METHODS=${METHODS:-"eevdf rustland proposed-no-admission proposed"}
CONFIGS=${CONFIGS:-"light saturated overload"}
ORDER_SEED=${ORDER_SEED:-104729}
COOLDOWN_S=${AFS_COOLDOWN_S:-2}

if [[ ${EUID:-$(id -u)} -eq 0 ]]; then
    echo "error: run matrix as the regular user, not root" >&2
    exit 2
fi

if [[ -d "$OUT_ROOT" ]] &&
   [[ -n "$(find "$OUT_ROOT" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null)" ]] &&
   [[ "${AFS_ALLOW_OVERWRITE:-0}" != "1" ]]; then
    echo "error: matrix output root is not empty: $OUT_ROOT" >&2
    exit 1
fi

PROJECT_DIRTY="$(git -C "$ROOT" status --porcelain | wc -l)"

if [[ "$PROJECT_DIRTY" -ne 0 ]] &&
   [[ "${AFS_ALLOW_DIRTY_BENCHMARK:-0}" != "1" ]]; then
    echo "error: project Git working tree is dirty" >&2
    echo "commit benchmark harness before final benchmark runs" >&2
    echo "for validation only, set AFS_ALLOW_DIRTY_BENCHMARK=1" >&2
    exit 1
fi

GENERATOR="$AFS_BIN_DIR/afs-workload-generator"

[[ -x "$GENERATOR" ]] || {
    echo "error: missing generator: $GENERATOR" >&2
    exit 1
}

mkdir -p "$OUT_ROOT/manifests"

export SCX_ROOT
export SCX_RUSTLAND_BIN

"$ROOT/scripts/collect_system_info.sh" \
    "$OUT_ROOT/environment-before.txt" >/dev/null

{
    echo "started_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "project_git_sha=$(git -C "$ROOT" rev-parse HEAD)"
    echo "project_git_branch=$(git -C "$ROOT" branch --show-current)"
    echo "project_git_dirty=$PROJECT_DIRTY"
    echo "kernel=$(uname -r)"
    echo "scx_root=$SCX_ROOT"

    if [[ -d "$SCX_ROOT/.git" ]]; then
        echo "scx_git_sha=$(git -C "$SCX_ROOT" rev-parse HEAD)"
    fi

    echo "scx_rustland_bin=$SCX_RUSTLAND_BIN"
    echo "repetitions=$REPETITIONS"
    echo "methods=$METHODS"
    echo "configs=$CONFIGS"
    echo "order_seed=$ORDER_SEED"
    echo "cooldown_s=$COOLDOWN_S"
} > "$OUT_ROOT/matrix-provenance.txt"

printf 'repetition\tconfig\torder\n' \
    > "$OUT_ROOT/execution-order.tsv"

printf 'repetition\tconfig\tsha256\tmanifest\n' \
    > "$OUT_ROOT/manifest-sha256.tsv"

read -r -a METHOD_ARRAY <<< "$METHODS"
read -r -a CONFIG_ARRAY <<< "$CONFIGS"

for rep in $(seq 0 $((REPETITIONS - 1))); do
    for cfg_index in "${!CONFIG_ARRAY[@]}"; do
        cfg=${CONFIG_ARRAY[$cfg_index]}
        config="$ROOT/configs/$cfg.toml"

        [[ -f "$config" ]] || {
            echo "error: config not found: $config" >&2
            exit 1
        }

        manifest_dir="$OUT_ROOT/manifests/$cfg"
        mkdir -p "$manifest_dir"

        rep_tag="$(printf '%02d' "$rep")"
        manifest="$manifest_dir/rep-$rep_tag.json"
        manifest_log="$manifest_dir/rep-$rep_tag-generation.log"

        if [[ ! -s "$manifest" ]]; then
            tmp_out="$OUT_ROOT/.manifest-generation-$cfg-$rep_tag"
            rm -rf "$tmp_out"

            "$GENERATOR" \
                --config "$config" \
                --output-dir "$tmp_out" \
                --method eevdf \
                --repetition "$rep" \
                --manifest "$manifest" \
                --dry-run \
                >"$manifest_log" 2>&1

            rm -rf "$tmp_out"
        fi

        manifest_sha="$(sha256sum "$manifest" | awk '{print $1}')"

        printf '%s\t%s\t%s\t%s\n' \
            "$rep" "$cfg" "$manifest_sha" "$manifest" \
            >> "$OUT_ROOT/manifest-sha256.tsv"

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

        printf '%s\t%s\t%s\n' \
            "$rep" "$cfg" "${ORDER[*]}" \
            >> "$OUT_ROOT/execution-order.tsv"

        for method in "${ORDER[@]}"; do
            out="$OUT_ROOT/$cfg/$method/rep-$rep_tag"

            echo
            echo "== $cfg / $method / repetition $rep =="

            "$ROOT/scripts/run_experiment.sh" \
                "$method" \
                "$config" \
                "$out" \
                "$rep" \
                "$manifest"

            "$ROOT/scripts/collect_system_info.sh" \
                "$out/environment-after.txt" >/dev/null

            if [[ "$COOLDOWN_S" != "0" ]]; then
                sleep "$COOLDOWN_S"
            fi
        done
    done
done

"$ROOT/scripts/collect_system_info.sh" \
    "$OUT_ROOT/environment-after.txt" >/dev/null

echo
echo "matrix completed cleanly: $OUT_ROOT"
