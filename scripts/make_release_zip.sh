#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION=$(tr -d '[:space:]' < "$ROOT/VERSION")
OUT=${1:-"$(dirname "$ROOT")/ai-fusion-scheduler-v${VERSION}.zip"}

"$ROOT/scripts/validate_package.sh"
find "$ROOT" -type d -name '__pycache__' -prune -exec rm -rf {} +
rm -f "$ROOT/PACKAGE_MANIFEST.txt" "$ROOT/SHA256SUMS.txt"

(
    cd "$ROOT"
    find . -type f \
        ! -path './target/*' \
        ! -path './results/*' \
        ! -path './run/*' \
        ! -path './.git/*' \
        ! -name 'PACKAGE_MANIFEST.txt' \
        ! -name 'SHA256SUMS.txt' \
        ! -name '*.pyc' \
        | sed 's#^./##' | LC_ALL=C sort > PACKAGE_MANIFEST.txt
    {
        while IFS= read -r file; do
            sha256sum "$file"
        done < PACKAGE_MANIFEST.txt
        sha256sum PACKAGE_MANIFEST.txt
    } > SHA256SUMS.txt
)

cd "$(dirname "$ROOT")"
rm -f "$OUT"
zip -qr "$OUT" "$(basename "$ROOT")" \
    -x '*/target/*' '*/results/*' '*/run/*' '*/__pycache__/*' '*.pyc' '.git/*'
unzip -t "$OUT" >/dev/null
sha256sum "$OUT" > "$OUT.sha256"
echo "$OUT"
