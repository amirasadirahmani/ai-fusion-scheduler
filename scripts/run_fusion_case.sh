#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
if [[ $# -lt 3 || $# -gt 4 ]]; then
    echo "usage: $0 METHOD CONFIG OUTPUT_DIR [REPETITION]" >&2
    exit 2
fi
METHOD=$1
CONFIG=$(realpath "$2")
OUTPUT=$(realpath -m "$3")
REPETITION=${4:-0}
mkdir -p "$OUTPUT"
cleanup() { stop_scheduler; }
trap cleanup EXIT INT TERM
case "$METHOD" in
    eevdf) ;;
    rustland) start_standard_rustland "$OUTPUT" ;;
    proposed) start_proposed_scheduler "$CONFIG" "$OUTPUT" ;;
    sched-deadline|sched_deadline) require_root_for_scheduler; METHOD=sched-deadline ;;
    *) echo "unknown method: $METHOD" >&2; exit 2 ;;
esac
PIPELINE="$AFS_BIN_DIR/afs-fusion-pipeline"
[[ -x "$PIPELINE" ]] || { echo "error: $PIPELINE missing; run scripts/build_userspace.sh" >&2; exit 1; }
"$PIPELINE" \
    --config "$CONFIG" \
    --output-dir "$OUTPUT" \
    --method "$METHOD" \
    --repetition "$REPETITION"
