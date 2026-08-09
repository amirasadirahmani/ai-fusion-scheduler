#!/usr/bin/env bash
set -u

ROOT="$(pwd)"
STAMP="2026-08-09"
OUT="$ROOT/docs/checkpoints/$STAMP"

if [ ! -f "$ROOT/Cargo.toml" ] || [ ! -d "$ROOT/crates" ]; then
  echo "ERROR: run from the ai-fusion-scheduler repository root."
  exit 1
fi

mkdir -p "$OUT"

run_capture() {
  name="$1"
  shift
  {
    echo "\$ $*"
    "$@" 2>&1 || true
  } > "$OUT/$name"
}

{
  echo "AFS reproducibility checkpoint"
  echo "date_local=$(date '+%Y-%m-%d %H:%M:%S %z')"
  echo "pwd=$ROOT"
  echo "user=$(id -un)"
  echo "host=$(hostname)"
} > "$OUT/README.txt"

run_capture uname.txt uname -a
run_capture os-release.txt cat /etc/os-release
run_capture lscpu.txt lscpu
run_capture memory.txt free -h
run_capture rustc-version.txt rustc --version
run_capture cargo-version.txt cargo --version
run_capture rustup-show.txt rustup show
run_capture clang-version.txt clang --version
run_capture bpftool-version.txt bpftool version
run_capture pkg-config-libbpf.txt pkg-config --modversion libbpf
run_capture cmake-version.txt cmake --version
run_capture ninja-version.txt ninja --version
run_capture python-version.txt python3 --version
run_capture pip-version.txt python3 -m pip --version
run_capture sched-ext-state.txt cat /sys/kernel/sched_ext/state
run_capture sched-ext-ops.txt sh -c 'cat /sys/kernel/sched_ext/root/ops 2>/dev/null || true'
run_capture btf-vmlinux.txt sh -c 'ls -lh /sys/kernel/btf/vmlinux 2>/dev/null || true'

if [ -r /boot/config-"$(uname -r)" ]; then
  grep -E 'CONFIG_SCHED_CLASS_EXT|CONFIG_BPF=|CONFIG_BPF_SYSCALL|CONFIG_BPF_JIT|CONFIG_DEBUG_INFO_BTF' \
    /boot/config-"$(uname -r)" > "$OUT/kernel-config-relevant.txt" 2>&1 || true
elif [ -r /proc/config.gz ]; then
  zgrep -E 'CONFIG_SCHED_CLASS_EXT|CONFIG_BPF=|CONFIG_BPF_SYSCALL|CONFIG_BPF_JIT|CONFIG_DEBUG_INFO_BTF' \
    /proc/config.gz > "$OUT/kernel-config-relevant.txt" 2>&1 || true
else
  echo "Kernel config file not directly readable." > "$OUT/kernel-config-relevant.txt"
fi

if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git rev-parse HEAD > "$OUT/project-git-head.txt" 2>&1 || true
  git status --short > "$OUT/project-git-status-short.txt" 2>&1 || true
  git status > "$OUT/project-git-status.txt" 2>&1 || true
  git diff --stat > "$OUT/project-git-diff-stat.txt" 2>&1 || true
  git diff > "$OUT/project-git-diff.patch" 2>&1 || true
  git diff --cached > "$OUT/project-git-diff-cached.patch" 2>&1 || true
fi

if [ -d "$HOME/scx/.git" ]; then
  git -C "$HOME/scx" rev-parse HEAD > "$OUT/scx-git-head.txt" 2>&1 || true
  git -C "$HOME/scx" status --short > "$OUT/scx-git-status-short.txt" 2>&1 || true
else
  echo "$HOME/scx is not a Git repository" > "$OUT/scx-git-head.txt"
fi

{
  for f in Cargo.toml Cargo.lock rust-toolchain.toml configs/smoke.toml; do
    if [ -f "$f" ]; then
      sha256sum "$f"
    fi
  done
} > "$OUT/key-file-sha256.txt"

{
  for f in \
    target/release/afs-rustland-scheduler \
    target/release/afs-workload-generator \
    target/release/afs-policy-simulator \
    "$HOME/scx/target/release/scx_rustland"
  do
    if [ -f "$f" ]; then
      sha256sum "$f"
    fi
  done
} > "$OUT/release-binary-sha256.txt"

find configs -maxdepth 2 -type f -print 2>/dev/null | sort > "$OUT/config-inventory.txt" || true
find scripts -maxdepth 2 -type f -print 2>/dev/null | sort > "$OUT/script-inventory.txt" || true
find results -maxdepth 2 -type f -print 2>/dev/null | sort > "$OUT/result-inventory.txt" || true

for src in \
  results/smoke-no-admission/summary.json \
  results/smoke-admission-v2/summary.json \
  results/smoke-admission-v2/tasks.csv \
  run/smoke-admission-v2/scheduler-stats.json
do
  if [ -f "$src" ]; then
    cp "$src" "$OUT/$(echo "$src" | tr '/' '-')"
  fi
done

{
  echo "Checkpoint captured at: $OUT"
  echo
  echo "Review next:"
  echo "  git status"
  echo "  git diff"
  echo
  echo "Suggested commit after review:"
  echo '  git commit -m "checkpoint: working sched_ext AFS with admission v2"'
} | tee "$OUT/NEXT_STEPS.txt"

echo
echo "Done: $OUT"
