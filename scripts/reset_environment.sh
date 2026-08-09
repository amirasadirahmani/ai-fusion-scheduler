#!/usr/bin/env bash
set -euo pipefail
pkill -INT -f '(^|/)afs-rustland-scheduler([[:space:]]|$)' 2>/dev/null || true
pkill -INT -f '(^|/)scx_rustland([[:space:]]|$)' 2>/dev/null || true
sleep 0.3
rm -rf /tmp/ai-fusion-scheduler/registry
mkdir -p /tmp/ai-fusion-scheduler/registry
if [[ -r /sys/kernel/sched_ext/state ]]; then
    echo "sched_ext state: $(cat /sys/kernel/sched_ext/state)"
fi
