# Frozen experiment environment

Fill this file on the target host before collecting final results.

| Item | Frozen value |
|---|---|
| Kernel release | Linux 6.12.95 |
| Kernel config SHA-256 | TODO |
| Distribution | TODO |
| `scx` release/tag | TODO |
| `scx` commit SHA | TODO |
| `scx_rustland_core` crate | 2.4.13 (default pin; change only if required) |
| Rust | TODO |
| Cargo | TODO |
| Clang | TODO |
| bpftool | TODO |
| libbpf | TODO |
| CPU model | TODO |
| Physical cores / logical CPUs | TODO |
| RAM | TODO |
| CPU governor | TODO |
| Bare metal or VM | TODO |
| Hypervisor | TODO / N/A |
| Thermal policy | TODO |
| Date frozen | TODO |

Do not change the kernel, `scx` pin, compiler, governor, CPU affinity policy or benchmark configuration after the first final experiment. A change requires restarting the full experiment matrix.

<!-- CHECKPOINT-2026-08-09:START -->
## Environment checkpoint — 2026-08-09

### Host / VM

- Host: Apple MacBook M1 Pro
- Hypervisor: UTM
- Guest: Debian 13.6 Trixie, ARM64
- Approx. resources: 6 vCPU, ~7.7 GiB RAM

### Kernel

```text
7.1.3+deb13-arm64
Debian 7.1.3-1~bpo13+1
```

Confirmed:

```text
CONFIG_SCHED_CLASS_EXT=y
CONFIG_BPF=y
CONFIG_BPF_SYSCALL=y
CONFIG_BPF_JIT=y
CONFIG_DEBUG_INFO_BTF=y
/sys/kernel/btf/vmlinux exists
```

Initial kernel `6.12.95` برای این پروژه کنار گذاشته شد چون sched_ext فعال نبود.

### Toolchain

- Rust `1.97.1`
- clang `19.1.7`
- bpftool `v7.5.0`
- libbpf `1.5.0`
- cmake `3.31.6`
- ninja `1.12.1`
- Python `3.13.5`
- pip `25.1.1`

### scx

```text
~/scx
7a58a3a1a857fa748ea5a3bda5ad312afbbeca5b
```

Stock RustLand:

```text
RustLand 1.1.2
scx_rustland_core 2.4.13
```

### Project

```text
~/project/ai-fusion-scheduler-v0.2.0/ai-fusion-scheduler
```

Compatibility pins:

```toml
rust-version = "1.88"
libc = "=0.2.186"
toml = "=0.8.23"
```

```toml
channel = "1.97.1"
```

### sched_ext

Idle:

```text
disabled
```

AFS ops while active:

```text
afs_rustland_0.1.0_aarch64_unknown_linux_gnu
```

### Registry rule

```bash
mkdir -p /tmp/ai-fusion-scheduler/registry
rm -f /tmp/ai-fusion-scheduler/registry/*
```

Registry معمولی را با `sudo mkdir` نسازید.

### Passwordless sudo

Lab VM:

```text
amir ALL=(ALL:ALL) NOPASSWD: ALL
```

### Reproducibility

قبل از benchmark نهایی، `scripts/capture_reproducibility_checkpoint.sh` اجرا و خروجی نگهداری شود.
<!-- CHECKPOINT-2026-08-09:END -->
