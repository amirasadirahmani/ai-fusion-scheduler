#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")" && pwd)/lib.sh"

if [[ $# -lt 3 || $# -gt 5 ]]; then
    echo "usage: $0 METHOD CONFIG OUTPUT_DIR [REPETITION] [MANIFEST]" >&2
    exit 2
fi

METHOD=$1
CONFIG=$(realpath "$2")
OUTPUT=$(realpath -m "$3")
REPETITION=${4:-0}
MANIFEST=${5:-}

if [[ "$METHOD" != "sched-deadline" && "$METHOD" != "sched_deadline" ]] &&
   [[ ${EUID:-$(id -u)} -eq 0 ]]; then
    echo "error: run EEVDF/RustLand/Proposed experiments as the regular user, not root" >&2
    echo "the harness uses passwordless sudo only for the scheduler process" >&2
    exit 2
fi

if [[ -d "$OUTPUT" ]] &&
   [[ -n "$(find "$OUTPUT" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null)" ]] &&
   [[ "${AFS_ALLOW_OVERWRITE:-0}" != "1" ]]; then
    echo "error: output directory is not empty: $OUTPUT" >&2
    echo "set AFS_ALLOW_OVERWRITE=1 only for deliberate reruns" >&2
    exit 1
fi

mkdir -p "$OUTPUT"

configure_cpu_partition "$CONFIG"
wait_for_cpu_quiescence


if [[ -n "$MANIFEST" ]]; then
    MANIFEST="$(realpath "$MANIFEST")"

    [[ -s "$MANIFEST" ]] || {
        echo "error: shared manifest does not exist or is empty: $MANIFEST" >&2
        exit 1
    }

    cp "$MANIFEST" "$OUTPUT/input-task-manifest.json"
    sha256sum "$MANIFEST" > "$OUTPUT/input-task-manifest.sha256"
fi

{
    echo "method=$METHOD"
    echo "config=$CONFIG"
    echo "config_sha256=$(sha256sum "$CONFIG" | awk '{print $1}')"
    echo "repetition=$REPETITION"
    echo "manifest=${MANIFEST:-generated-by-run}"
    echo "kernel=$(uname -r)"
    echo "workload_cpu_list=$AFS_WORKLOAD_CPU_LIST"
    echo "control_cpu_list=$AFS_CONTROL_CPU_LIST"
    echo "cpu_psi_some_avg10_before=$(cpu_psi_avg10)"
    echo "admission_psi_scope=workload-cgroup-v1"

    if git -C "$AFS_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        echo "project_git_sha=$(git -C "$AFS_ROOT" rev-parse HEAD)"
        echo "project_git_branch=$(git -C "$AFS_ROOT" branch --show-current)"
        echo "project_git_dirty=$(git -C "$AFS_ROOT" status --porcelain | wc -l)"
    fi

    if [[ -d "$SCX_ROOT/.git" ]]; then
        echo "scx_git_sha=$(git -C "$SCX_ROOT" rev-parse HEAD)"
    fi

    if [[ -f "$AFS_BIN_DIR/afs-workload-generator" ]]; then
        echo "generator_sha256=$(sha256sum "$AFS_BIN_DIR/afs-workload-generator" | awk '{print $1}')"
    fi

    if [[ -f "$AFS_BIN_DIR/afs-rustland-scheduler" ]]; then
        echo "proposed_scheduler_sha256=$(sha256sum "$AFS_BIN_DIR/afs-rustland-scheduler" | awk '{print $1}')"
    fi

    if [[ -f "$SCX_RUSTLAND_BIN" ]]; then
        echo "rustland_sha256=$(sha256sum "$SCX_RUSTLAND_BIN" | awk '{print $1}')"
    fi
} > "$OUTPUT/run-provenance.txt"

cleanup() {
    stop_scheduler >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

assert_sched_ext_disabled

NO_ADMISSION=0

case "$METHOD" in
    eevdf)
        ;;
    rustland)
        start_standard_rustland "$OUTPUT"
        ;;
    proposed)
        start_proposed_scheduler "$CONFIG" "$OUTPUT"
        ;;
    proposed-no-admission|proposed_no_admission)
        # Matrix-visible experimental variant:
        # run the same AFS scheduler while disabling only admission control.
        #
        # Keep METHOD=proposed for the generator, result schema, and validator,
        # while run-provenance (written above) retains the requested variant.
        start_proposed_scheduler "$CONFIG" "$OUTPUT"
        METHOD=proposed
        NO_ADMISSION=1
        ;;
    sched-deadline|sched_deadline)
        require_root_for_scheduler
        METHOD=sched-deadline
        ;;
    *)
        echo "unknown method: $METHOD" >&2
        exit 2
        ;;
esac

GENERATOR="$AFS_BIN_DIR/afs-workload-generator"

[[ -x "$GENERATOR" ]] || {
    echo "error: $GENERATOR missing; run scripts/build_userspace.sh" >&2
    exit 1
}

GEN_ARGS=(
    --config "$CONFIG"
    --output-dir "$OUTPUT"
    --method "$METHOD"
    --repetition "$REPETITION"
)

if [[ -n "$MANIFEST" ]]; then
    GEN_ARGS+=(--manifest "$MANIFEST")
fi

if [[ "$NO_ADMISSION" == "1" ]]; then
    GEN_ARGS+=(--no-admission)
fi

set +e
# shellcheck disable=SC2086
systemd-run --user --scope --quiet -p Delegate=yes -- \
    env AFS_WORKLOAD_CGROUP_SCOPED=1 \
    timeout -k 10s "${AFS_RUN_TIMEOUT_S:-600}s" \
    taskset -c "$AFS_CONTROL_CPU_LIST" \
    "$GENERATOR" \
    "${GEN_ARGS[@]}" \
    ${AFS_GENERATOR_ARGS:-}
GEN_RC=$?
set -e

stop_scheduler
trap - EXIT INT TERM

if [[ "$(sched_ext_state)" != "disabled" ]]; then
    echo "error: sched_ext was not disabled after experiment cleanup" >&2
    exit 1
fi

if [[ $GEN_RC -ne 0 ]]; then
    echo "error: workload generator exited with status $GEN_RC" >&2
    exit "$GEN_RC"
fi

[[ -s "$OUTPUT/summary.json" ]] || {
    echo "error: missing summary.json after successful generator run" >&2
    exit 1
}

[[ -s "$OUTPUT/tasks.csv" ]] || {
    echo "error: missing tasks.csv after successful generator run" >&2
    exit 1
}

VALIDATION_MANIFEST="$MANIFEST"

if [[ -z "$VALIDATION_MANIFEST" ]] &&
   [[ -s "$OUTPUT/task-manifest.json" ]]; then
    VALIDATION_MANIFEST="$OUTPUT/task-manifest.json"
fi

VALIDATE_ARGS=(
    "$OUTPUT"
    --expected-method "$METHOD"
)

if [[ -n "$VALIDATION_MANIFEST" ]]; then
    VALIDATE_ARGS+=(--manifest "$VALIDATION_MANIFEST")
fi

python3 "$AFS_ROOT/scripts/validate_run.py" "${VALIDATE_ARGS[@]}"

echo "experiment completed cleanly: method=$METHOD repetition=$REPETITION"
