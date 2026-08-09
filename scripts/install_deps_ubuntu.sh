#!/usr/bin/env bash
set -euo pipefail
MODE=${1:---print}
APT_PACKAGES=(
    build-essential clang llvm libelf-dev libz-dev libzstd-dev
    libbpf-dev bpftool pkg-config git python3 python3-pip linux-tools-common
)
case "$MODE" in
    --print)
        echo "sudo apt-get update"
        printf 'sudo apt-get install -y'
        printf ' %q' "${APT_PACKAGES[@]}"
        echo
        cat <<'MSG'
Install Rust >=1.82 with the official rustup installer, then verify:
  rustc --version
  cargo --version
Do not change the toolchain after final experiments begin.
MSG
        ;;
    --execute)
        sudo apt-get update
        sudo apt-get install -y "${APT_PACKAGES[@]}"
        ;;
    *) echo "usage: $0 [--print|--execute]" >&2; exit 2 ;;
esac
