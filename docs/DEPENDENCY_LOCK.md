# Dependency Lock و محیط بازتولید

این سند نسخه های مرجع پروژه را ثبت می کند. چون بخش `sched_ext` به کرنل و BPF وابسته است، علاوه بر وابستگی های نرم افزاری، نسخه کرنل و Commit مخزن `scx` نیز باید در گزارش نهایی ثبت شوند.

## 1. سیستم عامل مرجع Linux

```text
OS reference: Ubuntu 24.04 LTS یا Debian سازگار با Linux 6.12.95
Kernel reference: Linux 6.12.95
CPU scheduling scope: single Linux node
GPU/CUDA: Not used
PyTorch: Not used
```

گزینه های کرنل مورد نیاز:

```text
CONFIG_BPF=y
CONFIG_BPF_SYSCALL=y
CONFIG_BPF_JIT=y
CONFIG_DEBUG_INFO_BTF=y
CONFIG_SCHED_CLASS_EXT=y
CONFIG_CGROUPS=y
CONFIG_PSI=y
```

## 2. Rust

```text
Rust toolchain: 1.82.0
Edition: 2021
Cargo resolver: 2
```

فایل `rust-toolchain.toml` همین نسخه را Pin می کند.

## 3. وابستگی های Rust سطح Workspace

نسخه ها در `Cargo.toml` به صورت exact pin ثبت شده اند:

```text
anyhow = 1.0.95
clap = 4.5.23
csv = 1.3.1
ctrlc = 3.4.5
libc = 0.2.169
rand = 0.8.5
rand_chacha = 0.3.1
rand_distr = 0.4.3
serde = 1.0.217
serde_json = 1.0.134
thiserror = 1.0.69
toml = 0.8.19
tracing = 0.1.41
tracing-subscriber = 0.3.19
```

وابستگی های بخش `rustland-scheduler`:

```text
scx_rustland_core = 2.4.13
scx_cargo = 1.1.2
libbpf-rs = 0.26.2
```

در محیط هدف، پس از نصب Rust، این دستور باید اجرا شود تا `Cargo.lock` واقعی با checksumهای crates.io تولید شود:

```bash
cargo generate-lockfile
```

پس از تولید، فایل `Cargo.lock` را Commit کن و دیگر در طول آزمایش ها تغییر نده.

## 4. Python

```text
Python reference: 3.12.x
Python fallback tested syntax: 3.11+
```

وابستگی های تحلیل در `analysis/requirements-lock.txt` آمده اند.

## 5. ابزارهای Linux/BPF

نسخه دقیق این ابزارها باید با `scripts/collect_system_info.sh` ثبت شود:

```text
clang
llvm
bpftool
libbpf
pkg-config
make
cmake
gcc
```

## 6. scx Commit Lock

در هفته اول اجرای پروژه، یک Commit ثابت از مخزن `sched-ext/scx` انتخاب شود و در فایل های زیر ثبت گردد:

```text
ENVIRONMENT.md
results/environment-before.txt
docs/VALIDATION_STATUS.md
گزارش نهایی
```

نمونه ثبت:

```text
scx repository: https://github.com/sched-ext/scx
scx commit SHA: <to be filled on target system>
scx_rustland_core crate: 2.4.13
```

## 7. CUDA و PyTorch

```text
CUDA: Not used
PyTorch: Not used
GPU scheduling: خارج از محدوده پروژه
```

اگر در آینده Case Study مبتنی بر مدل GPU اضافه شود، باید یک Lock جداگانه برای CUDA، Driver، PyTorch و مدل استفاده شده تهیه شود. نسخه فعلی پروژه به هیچ کدام وابسته نیست.

## 8. بازتولید آزمایش

برای بازتولید کامل نتایج نهایی باید این فایل ها همراه گزارش منتشر شوند:

```text
Cargo.lock
analysis/requirements-lock.txt
ENVIRONMENT.md
configs/*.toml
results/real/**
results/summary/aggregate-summary.csv
scripts/collect_system_info.sh output
```
