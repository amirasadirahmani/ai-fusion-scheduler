#!/usr/bin/env bash
set -euo pipefail

AFS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AFS_BIN_DIR="${AFS_BIN_DIR:-$AFS_ROOT/target/release}"
AFS_SCHED_PID=""

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "error: required command not found: $1" >&2
        return 1
    }
}

require_root_for_scheduler() {
    if [[ ${EUID:-$(id -u)} -ne 0 ]]; then
        echo "error: this scheduler mode requires root/CAP_SYS_NICE; rerun with sudo" >&2
        return 1
    fi
}

wait_for_sched_ext_state() {
    local expected="${1:-enabled}"
    local timeout_s="${2:-10}"
    local state_file=/sys/kernel/sched_ext/state
    local start=$SECONDS
    while (( SECONDS - start < timeout_s )); do
        if [[ -r "$state_file" ]] && [[ "$(tr -d '\n' < "$state_file")" == "$expected" ]]; then
            return 0
        fi
        if [[ -n "${AFS_SCHED_PID:-}" ]] && ! kill -0 "$AFS_SCHED_PID" 2>/dev/null; then
            echo "error: scheduler process exited before sched_ext became $expected" >&2
            return 1
        fi
        sleep 0.1
    done
    echo "error: timed out waiting for sched_ext state=$expected" >&2
    return 1
}

stop_scheduler() {
    if [[ -n "${AFS_SCHED_PID:-}" ]] && kill -0 "$AFS_SCHED_PID" 2>/dev/null; then
        kill -INT "$AFS_SCHED_PID" 2>/dev/null || true
        for _ in $(seq 1 50); do
            kill -0 "$AFS_SCHED_PID" 2>/dev/null || break
            sleep 0.1
        done
        if kill -0 "$AFS_SCHED_PID" 2>/dev/null; then
            kill -TERM "$AFS_SCHED_PID" 2>/dev/null || true
        fi
        wait "$AFS_SCHED_PID" 2>/dev/null || true
    fi
    AFS_SCHED_PID=""
}

start_proposed_scheduler() {
    local config=$1
    local output_dir=$2
    require_root_for_scheduler
    local binary="$AFS_BIN_DIR/afs-rustland-scheduler"
    [[ -x "$binary" ]] || {
        echo "error: $binary not found; run scripts/build_sched_ext.sh" >&2
        return 1
    }
    mkdir -p "$output_dir"
    # shellcheck disable=SC2086
    RUST_LOG="${RUST_LOG:-info}" "$binary" \
        --config "$config" \
        --partial true \
        --stats-json "$output_dir/scheduler-stats.json" \
        ${AFS_SCHEDULER_ARGS:-} \
        >"$output_dir/scheduler.log" 2>&1 &
    AFS_SCHED_PID=$!
    wait_for_sched_ext_state enabled 15
}

start_standard_rustland() {
    local output_dir=$1
    require_root_for_scheduler
    require_command scx_rustland
    mkdir -p "$output_dir"
    if ! scx_rustland --help 2>&1 | grep -q -- '--partial'; then
        echo "error: installed scx_rustland has no --partial option; refusing full-system baseline" >&2
        return 1
    fi
    scx_rustland --partial >"$output_dir/scx-rustland.log" 2>&1 &
    AFS_SCHED_PID=$!
    wait_for_sched_ext_state enabled 15
}
