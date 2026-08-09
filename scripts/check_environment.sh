#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

failures=0
warnings=0
check_command() {
    if command -v "$1" >/dev/null 2>&1; then
        printf '[ok]   %-24s %s\n' "$1" "$(command -v "$1")"
    else
        printf '[fail] %-24s missing\n' "$1"
        failures=$((failures + 1))
    fi
}
optional_command() {
    if command -v "$1" >/dev/null 2>&1; then
        printf '[ok]   %-24s %s\n' "$1" "$(command -v "$1")"
    else
        printf '[warn] %-24s missing\n' "$1"
        warnings=$((warnings + 1))
    fi
}

echo "AI Fusion Scheduler environment preflight"
echo "project: $ROOT"
echo "kernel:  $(uname -r)"
[[ "$(uname -r)" == 6.12.95* ]] || {
    echo "[warn] proposal freezes Linux 6.12.95; current kernel differs"
    warnings=$((warnings + 1))
}

for cmd in cargo rustc clang bpftool pkg-config git python3; do check_command "$cmd"; done
for cmd in jq taskset chrt perf; do optional_command "$cmd"; done

CONFIG_FILE="/boot/config-$(uname -r)"
if [[ -r "$CONFIG_FILE" ]]; then
    for option in CONFIG_BPF CONFIG_BPF_SYSCALL CONFIG_BPF_JIT CONFIG_DEBUG_INFO_BTF CONFIG_SCHED_CLASS_EXT; do
        if grep -q "^${option}=y" "$CONFIG_FILE"; then
            echo "[ok]   $option=y"
        else
            echo "[fail] $option is not y in $CONFIG_FILE"
            failures=$((failures + 1))
        fi
    done
else
    echo "[warn] cannot read $CONFIG_FILE"
    warnings=$((warnings + 1))
fi

if [[ -r /sys/kernel/btf/vmlinux ]]; then
    echo "[ok]   /sys/kernel/btf/vmlinux"
else
    echo "[fail] missing /sys/kernel/btf/vmlinux"
    failures=$((failures + 1))
fi

if [[ -r /sys/kernel/sched_ext/state ]]; then
    echo "[ok]   sched_ext state: $(cat /sys/kernel/sched_ext/state)"
else
    echo "[fail] /sys/kernel/sched_ext/state is unavailable"
    failures=$((failures + 1))
fi

if [[ -r /proc/pressure/cpu ]]; then
    echo "[ok]   CPU PSI available"
else
    echo "[fail] /proc/pressure/cpu unavailable"
    failures=$((failures + 1))
fi

echo "summary: failures=$failures warnings=$warnings"
(( failures == 0 ))
