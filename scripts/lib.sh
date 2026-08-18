#!/usr/bin/env bash
set -euo pipefail

AFS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AFS_BIN_DIR="${AFS_BIN_DIR:-$AFS_ROOT/target/release}"

SCX_ROOT="${SCX_ROOT:-$HOME/scx}"
SCX_RUSTLAND_BIN="${SCX_RUSTLAND_BIN:-$SCX_ROOT/target/release/scx_rustland}"
AFS_EXPECTED_SCX_SHA="${AFS_EXPECTED_SCX_SHA:-7a58a3a1a857fa748ea5a3bda5ad312afbbeca5b}"

AFS_SCHED_PID=""
AFS_LAUNCH_PID=""
AFS_SCHED_PID_FILE=""

AFS_WORKLOAD_CPU_LIST=""
AFS_CONTROL_CPU_LIST=""

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "error: required command not found: $1" >&2
        return 1
    }
}


configure_cpu_partition() {
    local config=$1
    local partition

    partition="$(python3 - "$config" <<'PYCPU'
import sys
import tomllib
from pathlib import Path

config = sys.argv[1]

with open(config, "rb") as f:
    cfg = tomllib.load(f)

requested = int(cfg["experiment"]["cpus"])

status = Path("/proc/self/status").read_text()
line = next(
    x for x in status.splitlines()
    if x.startswith("Cpus_allowed_list:")
)
allowed_spec = line.split(":", 1)[1].strip()

def parse_cpu_list(spec):
    cpus = []
    for part in spec.split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            lo, hi = map(int, part.split("-", 1))
            cpus.extend(range(lo, hi + 1))
        else:
            cpus.append(int(part))
    return sorted(set(cpus))

allowed = parse_cpu_list(allowed_spec)

if requested <= 0:
    raise SystemExit("error: experiment.cpus must be positive")

if requested >= len(allowed):
    raise SystemExit(
        f"error: experiment requests {requested} workload CPUs, "
        f"but only {len(allowed)} CPUs are allowed; "
        "at least one control CPU must remain"
    )

# Keep lower-numbered CPUs for housekeeping/control and place
# experimental workload on the remaining higher-numbered CPUs.
workload = allowed[-requested:]
control = allowed[:-requested]

print(
    ",".join(map(str, workload))
    + "|"
    + ",".join(map(str, control))
)
PYCPU
)"

    IFS='|' read -r AFS_WORKLOAD_CPU_LIST AFS_CONTROL_CPU_LIST <<< "$partition"

    export AFS_WORKLOAD_CPU_LIST
    export AFS_CONTROL_CPU_LIST

    echo "cpu partition: workload=$AFS_WORKLOAD_CPU_LIST control=$AFS_CONTROL_CPU_LIST"
}

cpu_psi_avg10() {
    awk '
        /^some / {
            for (i = 1; i <= NF; i++) {
                if ($i ~ /^avg10=/) {
                    split($i, a, "=")
                    print a[2]
                    exit
                }
            }
        }
    ' /proc/pressure/cpu
}

wait_for_cpu_quiescence() {
    local threshold="${AFS_PSI_QUIESCENT_AVG10:-5.0}"
    local timeout_s="${AFS_PSI_QUIESCENCE_TIMEOUT_S:-90}"
    local start=$SECONDS
    local value

    while (( SECONDS - start < timeout_s )); do
        value="$(cpu_psi_avg10)"

        if awk -v v="$value" -v t="$threshold" \
            'BEGIN { exit !(v <= t) }'
        then
            echo "cpu quiescent: psi_some_avg10=$value threshold=$threshold"
            return 0
        fi

        sleep 1
    done

    echo "error: CPU PSI did not become quiescent" >&2
    echo "current avg10=$(cpu_psi_avg10), threshold=$threshold" >&2
    return 1
}

require_passwordless_sudo() {
    if ! sudo -n true >/dev/null 2>&1; then
        echo "error: passwordless sudo is required for scheduler lifecycle" >&2
        return 1
    fi
}

require_root_for_scheduler() {
    if [[ ${EUID:-$(id -u)} -ne 0 ]]; then
        echo "error: this mode requires root/CAP_SYS_NICE" >&2
        return 1
    fi
}

sched_ext_state() {
    cat /sys/kernel/sched_ext/state 2>/dev/null || echo unavailable
}

assert_sched_ext_disabled() {
    local state
    state="$(sched_ext_state)"
    if [[ "$state" != "disabled" ]]; then
        echo "error: expected sched_ext=disabled before experiment, got: $state" >&2
        return 1
    fi
}

scheduler_alive() {
    [[ -n "${AFS_SCHED_PID:-}" ]] &&
        sudo -n kill -0 "$AFS_SCHED_PID" 2>/dev/null
}

wait_for_sched_ext_state() {
    local expected="${1:-enabled}"
    local timeout_s="${2:-10}"
    local start=$SECONDS
    local state

    while (( SECONDS - start < timeout_s )); do
        state="$(sched_ext_state)"

        if [[ "$state" == "$expected" ]]; then
            return 0
        fi

        if [[ "$expected" == "enabled" ]] &&
           [[ -n "${AFS_SCHED_PID:-}" ]] &&
           ! scheduler_alive; then
            echo "error: scheduler exited before sched_ext became enabled" >&2
            return 1
        fi

        sleep 0.1
    done

    echo "error: timed out waiting for sched_ext state=$expected; current=$(sched_ext_state)" >&2
    return 1
}

verify_scx_checkout() {
    [[ -x "$SCX_RUSTLAND_BIN" ]] || {
        echo "error: pinned RustLand binary not found: $SCX_RUSTLAND_BIN" >&2
        return 1
    }

    if [[ -d "$SCX_ROOT/.git" ]]; then
        local actual
        actual="$(git -C "$SCX_ROOT" rev-parse HEAD)"

        if [[ "$actual" != "$AFS_EXPECTED_SCX_SHA" ]] &&
           [[ "${AFS_ALLOW_SCX_MISMATCH:-0}" != "1" ]]; then
            echo "error: scx checkout mismatch" >&2
            echo "expected: $AFS_EXPECTED_SCX_SHA" >&2
            echo "actual:   $actual" >&2
            return 1
        fi
    fi
}

launch_privileged_scheduler() {
    local log_file=$1
    local pid_file=$2
    shift 2

    require_passwordless_sudo
    require_command taskset
    mkdir -p "$(dirname "$log_file")"

    rm -f "$pid_file" 2>/dev/null || sudo -n rm -f "$pid_file"

    sudo -n sh -c '
        pidfile=$1
        control_cpus=$2
        shift 2
        umask 022
        printf "%s\n" "$$" > "$pidfile"
        exec taskset -c "$control_cpus" "$@"
    ' sh "$pid_file" "$AFS_CONTROL_CPU_LIST" "$@" >"$log_file" 2>&1 &

    AFS_LAUNCH_PID=$!
    AFS_SCHED_PID_FILE="$pid_file"

    for _ in $(seq 1 100); do
        if [[ -s "$pid_file" ]]; then
            AFS_SCHED_PID="$(tr -d '\n' < "$pid_file")"
            break
        fi

        if ! kill -0 "$AFS_LAUNCH_PID" 2>/dev/null; then
            wait "$AFS_LAUNCH_PID" 2>/dev/null || true
            echo "error: privileged scheduler launcher exited early" >&2
            tail -40 "$log_file" >&2 || true
            return 1
        fi

        sleep 0.05
    done

    if [[ -z "$AFS_SCHED_PID" ]]; then
        echo "error: scheduler PID was not captured" >&2
        tail -40 "$log_file" >&2 || true
        return 1
    fi
}

stop_scheduler() {
    if [[ -n "${AFS_SCHED_PID:-}" ]] && scheduler_alive; then
        sudo -n kill -INT "$AFS_SCHED_PID" 2>/dev/null || true

        for _ in $(seq 1 50); do
            scheduler_alive || break
            sleep 0.1
        done

        if scheduler_alive; then
            sudo -n kill -TERM "$AFS_SCHED_PID" 2>/dev/null || true

            for _ in $(seq 1 30); do
                scheduler_alive || break
                sleep 0.1
            done
        fi

        if scheduler_alive; then
            echo "warning: scheduler ignored INT/TERM; sending KILL" >&2
            sudo -n kill -KILL "$AFS_SCHED_PID" 2>/dev/null || true
        fi
    fi

    if [[ -n "${AFS_LAUNCH_PID:-}" ]]; then
        wait "$AFS_LAUNCH_PID" 2>/dev/null || true
    fi

    wait_for_sched_ext_state disabled 15

    if [[ -n "${AFS_SCHED_PID_FILE:-}" ]]; then
        rm -f "$AFS_SCHED_PID_FILE" 2>/dev/null ||
            sudo -n rm -f "$AFS_SCHED_PID_FILE" 2>/dev/null ||
            true
    fi

    AFS_SCHED_PID=""
    AFS_LAUNCH_PID=""
    AFS_SCHED_PID_FILE=""
}

start_proposed_scheduler() {
    local config=$1
    local output_dir=$2
    local binary="$AFS_BIN_DIR/afs-rustland-scheduler"

    [[ -x "$binary" ]] || {
        echo "error: $binary not found; run scripts/build_sched_ext.sh" >&2
        return 1
    }

    assert_sched_ext_disabled
    mkdir -p "$output_dir"

    launch_privileged_scheduler \
        "$output_dir/scheduler.log" \
        "$output_dir/.scheduler.pid" \
        env "RUST_LOG=${RUST_LOG:-info}" \
        "$binary" \
        --config "$config" \
        --partial true \
        --stats-json "$output_dir/scheduler-stats.json" \
        ${AFS_SCHEDULER_ARGS:-}

    wait_for_sched_ext_state enabled 15
}

start_standard_rustland() {
    local output_dir=$1

    verify_scx_checkout
    assert_sched_ext_disabled

    if ! "$SCX_RUSTLAND_BIN" --help 2>&1 | grep -q -- '--partial'; then
        echo "error: pinned scx_rustland has no --partial option" >&2
        return 1
    fi

    mkdir -p "$output_dir"

    launch_privileged_scheduler \
        "$output_dir/scx-rustland.log" \
        "$output_dir/.scheduler.pid" \
        "$SCX_RUSTLAND_BIN" \
        --partial

    wait_for_sched_ext_state enabled 15
}
