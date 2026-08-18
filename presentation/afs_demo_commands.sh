#!/usr/bin/env bash
set -euo pipefail

cd ~/project/ai-fusion-scheduler-v0.2.0/ai-fusion-scheduler

echo "=== AFS FINAL DEMO ==="
printf 'branch:    '; git branch --show-current
printf 'HEAD:      '; git rev-parse --short HEAD
printf 'kernel:    '; uname -r
printf 'sched_ext: '; cat /sys/kernel/sched_ext/state
echo

echo "=== GIT STATUS ==="
git status --short
echo

echo "=== RECENT COMMITS ==="
git log -6 --oneline --decorate
echo

echo "=== PRIMARY RESULT HIGHLIGHTS ==="
grep -n -E 'Light|Saturated|Overload|82\.17|13\.32|79\.38' \
  docs/final-results/FINAL_RESULTS_SUMMARY.md || true
echo

echo "=== PUBLICATION PLOTS ==="
printf 'PNG count: '
find docs/secondary-results/plots -type f -name '*.png' | wc -l
echo

echo "=== SECONDARY INTEGRITY ==="
cat docs/secondary-results/SECONDARY_INTEGRITY.md
echo

echo "=== FUSION SUMMARY ==="
cat docs/secondary-results/FUSION_SUMMARY.md
echo

echo "=== PUBLICATION ARTIFACTS ==="
ls -lh \
  docs/final-proposal-fa.pdf \
  report/FINAL_REPORT_FA.md \
  presentation/ai_fusion_scheduler_final_results_deck.pptx
