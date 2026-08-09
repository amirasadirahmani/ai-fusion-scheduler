#!/usr/bin/env bash
set -euo pipefail
# Long Rust binary names exceed Linux's 15-byte comm field, so match the full
# command line rather than relying on `pkill -x`.
pkill -KILL -f '(^|/)afs-rustland-scheduler([[:space:]]|$)' 2>/dev/null || true
pkill -KILL -f '(^|/)scx_rustland([[:space:]]|$)' 2>/dev/null || true
pkill -TERM -f '(^|/)afs-workload-worker([[:space:]]|$)' 2>/dev/null || true
if [[ ${1:-} == "--sysrq" ]]; then
    [[ ${EUID:-$(id -u)} -eq 0 ]] || { echo "--sysrq requires root" >&2; exit 1; }
    [[ -w /proc/sysrq-trigger ]] || { echo "/proc/sysrq-trigger not writable" >&2; exit 1; }
    echo S > /proc/sysrq-trigger
fi
cat /sys/kernel/sched_ext/state 2>/dev/null || true
