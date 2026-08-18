#!/usr/bin/env bash
set -euo pipefail

OUT=${1:-results/environment.txt}
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCX_ROOT="${SCX_ROOT:-$HOME/scx}"
SCX_RUSTLAND_BIN="${SCX_RUSTLAND_BIN:-$SCX_ROOT/target/release/scx_rustland}"

mkdir -p "$(dirname "$OUT")"

{
    echo "collected_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "project_root=$ROOT"
    echo "uname=$(uname -a)"
    echo "kernel_release=$(uname -r)"

    echo "os_release_begin"
    cat /etc/os-release 2>/dev/null || true
    echo "os_release_end"

    for cmd in rustc cargo clang bpftool git python3; do
        if command -v "$cmd" >/dev/null 2>&1; then
            echo "$cmd=$($cmd --version 2>&1 | head -1)"
        else
            echo "$cmd=missing"
        fi
    done

    echo "lscpu_begin"
    lscpu 2>/dev/null || true
    echo "lscpu_end"

    echo "memory_begin"
    free -h 2>/dev/null || true
    echo "memory_end"

    echo "governors_begin"
    grep . /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor 2>/dev/null || true
    echo "governors_end"

    echo "sched_ext_state=$(cat /sys/kernel/sched_ext/state 2>/dev/null || echo unavailable)"
    echo "cpu_psi=$(tr '\n' ';' < /proc/pressure/cpu 2>/dev/null || echo unavailable)"

    CONFIG="/boot/config-$(uname -r)"

    if [[ -r "$CONFIG" ]]; then
        echo "kernel_config_sha256=$(sha256sum "$CONFIG" | awk '{print $1}')"
        grep -E '^CONFIG_(BPF|BPF_SYSCALL|BPF_JIT|BPF_JIT_ALWAYS_ON|BPF_JIT_DEFAULT_ON|DEBUG_INFO_BTF|SCHED_CLASS_EXT)=' \
            "$CONFIG" || true
    fi

    if git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        echo "project_git_sha=$(git -C "$ROOT" rev-parse HEAD)"
        echo "project_git_branch=$(git -C "$ROOT" branch --show-current)"
        echo "project_git_dirty=$(git -C "$ROOT" status --porcelain | wc -l)"
    fi

    if [[ -d "$SCX_ROOT/.git" ]]; then
        echo "scx_root=$SCX_ROOT"
        echo "scx_git_sha=$(git -C "$SCX_ROOT" rev-parse HEAD)"
    fi

    if [[ -x "$SCX_RUSTLAND_BIN" ]]; then
        echo "scx_rustland_path=$SCX_RUSTLAND_BIN"
        echo "scx_rustland_sha256=$(sha256sum "$SCX_RUSTLAND_BIN" | awk '{print $1}')"
        echo "scx_rustland_version=$("$SCX_RUSTLAND_BIN" --version 2>&1 | head -1 || true)"
    fi

    for binary in \
        "$ROOT/target/release/afs-workload-generator" \
        "$ROOT/target/release/afs-rustland-scheduler"
    do
        if [[ -f "$binary" ]]; then
            echo "binary_sha256[$binary]=$(sha256sum "$binary" | awk '{print $1}')"
        fi
    done
} > "$OUT"

echo "$OUT"
