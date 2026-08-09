#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
find scripts -type f -name '*.sh' -print0 | while IFS= read -r -d '' script; do
    bash -n "$script"
done
python3 -m py_compile analysis/*.py scripts/static_validate.py scripts/test_analysis.py
python3 scripts/static_validate.py
python3 scripts/test_analysis.py
if command -v cargo >/dev/null 2>&1; then
    cargo fmt --all -- --check
    cargo test --workspace --exclude afs-rustland-scheduler
else
    echo "warning: cargo unavailable; Rust compile/tests skipped" >&2
fi
echo "package validation completed"
