#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }
: "${BPF_CLANG:=clang}"
export BPF_CLANG
cargo build --release -p afs-rustland-scheduler
