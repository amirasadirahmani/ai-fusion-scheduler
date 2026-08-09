#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }
cargo generate-lockfile
sha256sum Cargo.lock > Cargo.lock.sha256
if [[ -n "${SCX_ROOT:-}" ]] && git -C "$SCX_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git -C "$SCX_ROOT" rev-parse HEAD > SCX_COMMIT
fi
