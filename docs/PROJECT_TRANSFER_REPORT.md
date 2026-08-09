# گزارش انتقال کامل پروژه AFS — Checkpoint 2026-08-09

> **وضعیت این سند:** مرجع انتقال و بازیابی وضعیت پروژه تا پایان اعتبارسنجی Smoke برای Admission v2
> **تاریخ:** 2026-08-09
> **پروژه:** AI Fusion Scheduler (AFS)
> **مسیر کاری فعلی:** `~/project/ai-fusion-scheduler-v0.2.0/ai-fusion-scheduler`
> **اصل مهم:** نتایج این سند عمدتاً Development/Smoke/Diagnostic هستند و نباید بدون اجرای پروتکل نهایی benchmark به‌عنوان نتایج نهایی پایان‌نامه/مقاله گزارش شوند.

---

## 1. هدف این Checkpoint

این سند برای جلوگیری از گم‌شدن تصمیم‌ها، تغییرات کد، نسخه‌های محیط، خطاهای حل‌شده، نتایج تشخیصی و برنامه ادامه پروژه ایجاد شده است.

از این نقطه به بعد، هر تغییر مهم باید یکی از این سه ردپا را داشته باشد:

1. **کد یا پیکربندی نسخه‌پذیر در Git**
2. **توضیح تصمیم در `DECISIONS.md` یا این گزارش**
3. **نتیجه قابل بازتولید در `results/` و/یا checkpoint محیط**

این Checkpoint پایان مرحله‌ی زیر است:

`Environment bring-up → sched_ext enablement → stock RustLand validation → AFS build/attach → real end-to-end smoke → Admission v1 false-rejection diagnosis → Admission v2 implementation + unit tests + real validation`

---

## 2. عنوان و دامنه پژوهش

### عنوان فارسی

**طراحی و ارزیابی یک زمان‌بند Soft-Deadline آگاه از کاربرد برای بارهای کاری هوش مصنوعی در لینوکس با استفاده از Rust و sched_ext**

### عنوان انگلیسی

**Design and Evaluation of a Rust-Based Application-Aware Soft-Deadline Scheduler for Artificial Intelligence Workloads Using Linux sched_ext**

### دامنه فعلی

- Linux `sched_ext`
- Rust userspace scheduler
- `scx_rustland_core`
- Application-aware metadata
- Soft deadlines
- Runtime estimation
- Admission control
- Priority + aging + laxity
- CPU / memory / I/O workload generation
- Massive Data Fusion case study
- Single-node evaluation

### خارج از دامنه فعلی

- GPU scheduling
- Kubernetes
- multi-node orchestration
- Reinforcement Learning
- hard real-time guarantees
- ادعای energy measurement با Intel RAPL روی VM فعلی

---

## 3. معماری مفهومی AFS

```text
Application / Workload
        |
        | metadata
        v
Metadata Registry
        |
        v
AFS Userspace Scheduler (Rust)
        |
        | score / ordering / slice decisions
        v
scx_rustland_core
        |
        v
Linux sched_ext / BPF struct_ops
        |
        v
Linux Scheduler
```

Metadata مورد استفاده:

- `task_id`
- `process_id`
- `workflow_id`
- `stage_id`
- `workload_class`
- `absolute_deadline`
- `estimated_remaining_runtime`
- `application_priority`
- `release_time`

کلاس‌ها:

- `Critical`
- `Interactive`
- `Batch`

---

## 4. مدل زمان اجرا و امتیازدهی

### 4.1 تخمین زمان باقی‌مانده

```text
R_hat_i(t) = max(0, C_hat_i - C_i,consumed(t))
```

در implementation فعلی:

```rust
remaining_ns =
    estimated_total_runtime_ns
    .saturating_sub(consumed_cpu_ns)
    .min(estimated_remaining_runtime_ns.max(1));
```

### 4.2 Laxity

برای task دارای application deadline:

```text
L_i(t) = D_i - t - R_hat_i(t)
```

در کد:

```rust
deadline - now_ns - remaining_ns
```

برای Batch بدون deadline، `batch_base_laxity_ms` استفاده می‌شود.

### 4.3 Priority bonus

```text
priority_bonus = alpha_ms * priority
```

### 4.4 Aging bonus

```text
normalized_age = clamp(waiting / aging_horizon, 0, 1)
aging_bonus = beta_ms * normalized_age
```

### 4.5 Score نهایی

```text
score = laxity - priority_bonus - aging_bonus
```

**Score کمتر = اولویت scheduling بالاتر**

Tie-break:

1. score
2. release time
3. task id

### 4.6 Slice بر اساس کلاس

- Critical → `critical_slice_ms`
- Interactive → `interactive_slice_ms`
- Batch → `batch_slice_ms`

---

## 5. Admission Controller

### 5.1 Admission v1

منطق اولیه:

```text
total_work = active_remaining + incoming_runtime
queue_delay = total_work / cpus
predicted_duration = queue_delay * safety_factor
```

اگر:

```text
now + predicted_duration > deadline
```

task با دلیل `predicted_infeasible` رد می‌شد.

### 5.2 دو ایراد شناسایی‌شده

1. runtime خود task بر تعداد CPU تقسیم می‌شد؛ برای worker CPU-bound تک‌وظیفه‌ای تخمین مناسبی نیست.
2. تمام remaining work فعال interference فرض می‌شد، حتی Batch کم‌اولویتی که policy واقعی آن را عقب می‌اندازد.

### 5.3 Admission v2

مدل اصلاح‌شده:

```text
base_duration =
    own_runtime
    + relevant_interference / cpus

predicted_duration =
    base_duration * safety_factor
```

`relevant_interference` فقط از taskهایی تشکیل می‌شود که با **همان score policy اصلی scheduler** در وضعیت فعلی، no-later-than incoming task رتبه می‌گیرند.

اهداف:

- کاهش False Rejection
- حفظ Reject برای workload واقعاً infeasible
- سازگاری Admission با scheduler policy
- یکسان‌سازی منطق generator واقعی و simulator

### 5.4 PSI

- feasibility/deadline check برای Critical هم اعمال می‌شود.
- exemption کلاس Critical مربوط به PSI rejection/delay است.
- Critical به‌صورت unconditional admit پیاده‌سازی نشده است.

---

## 6. محیط اجرایی تثبیت‌شده

### Host

- Apple MacBook M1 Pro
- UTM virtualization

### Guest VM

- Debian 13.6 Trixie
- ARM64 / `aarch64`
- حدود 6 vCPU
- حدود 7.7 GiB RAM

### Kernel اولیه

```text
6.12.95
```

مشکل:

```text
CONFIG_SCHED_CLASS_EXT is not set
```

### Kernel فعلی

```text
Linux kernel-lab 7.1.3+deb13-arm64
#1 SMP PREEMPT Debian 7.1.3-1~bpo13+1 (2026-07-12) aarch64
```

موارد تأییدشده:

```text
CONFIG_SCHED_CLASS_EXT=y
CONFIG_BPF=y
CONFIG_BPF_SYSCALL=y
CONFIG_BPF_JIT=y
CONFIG_DEBUG_INFO_BTF=y
/sys/kernel/btf/vmlinux exists
```

Idle state:

```text
/sys/kernel/sched_ext/state = disabled
```

### Toolchain

- clang 19.1.7
- bpftool v7.5.0
- libbpf pkg 1.5.0
- cmake 3.31.6
- ninja 1.12.1
- Python 3.13.5
- pip 25.1.1
- Rust toolchain 1.97.1

Packages مهم:

- `libelf-dev`
- `libbpf-dev`
- `libseccomp-dev`

---

## 7. Passwordless sudo برای VM آزمایشگاهی

تنظیم‌شده:

```text
amir ALL=(ALL:ALL) NOPASSWD: ALL
```

مسیر:

```text
/etc/sudoers.d/99-amir-nopasswd
```

Validation موفق:

```text
/etc/sudoers.d/99-amir-nopasswd: parsed OK
root
```

هدف: اجرای آزمایش و فیلم‌برداری بدون prompt پسورد.

---

## 8. Repository رسمی scx

مسیر:

```text
~/scx
```

Commit تثبیت‌شده:

```text
7a58a3a1a857fa748ea5a3bda5ad312afbbeca5b
```

در benchmark نهایی بدون ثبت نسخه `git pull` انجام نشود.

---

## 9. Stock RustLand validation

```text
RustLand version 1.1.2
scx_rustland_core 2.4.13
```

Binary:

```text
~/scx/target/release/scx_rustland
```

Validation:

```text
STATE WHILE RUNNING = enabled
STATE AFTER STOP = disabled
EXIT: unregistered from user space
Unregister RustLand scheduler
```

stress-ng با 4 CPU و 10 ثانیه بدون failure اجرا شد.

---

## 10. ساختار پروژه AFS

مسیر:

```text
~/project/ai-fusion-scheduler-v0.2.0/ai-fusion-scheduler
```

Root مشاهده‌شده:

```text
analysis
Cargo.toml
CHANGELOG.md
configs
crates
DECISIONS.md
docs
ENVIRONMENT.md
LICENSE
Makefile
PACKAGE_CONTENTS_FA.md
PACKAGE_MANIFEST.txt
presentation
PROJECT_CHECKLIST.md
README.md
report
results
rust-toolchain.toml
scripts
SHA256SUMS.txt
VERSION
```

Workspace:

```text
crates/common
crates/metadata-manager
crates/policy-core
crates/policy-simulator
crates/workload-worker
crates/workload-generator
crates/fusion-pipeline
crates/rustland-scheduler
```

---

## 11. تغییرات dependency و toolchain

Root `Cargo.toml`:

```toml
rust-version = "1.88"
libc = "=0.2.186"
toml = "=0.8.23"
```

`rust-toolchain.toml`:

```toml
channel = "1.97.1"
```

اجرا شده:

```bash
cargo update -p toml --precise 0.8.23
```

---

## 12. تغییر `common`

در `crates/common/src/lib.rs`:

```rust
pub use linux::*;
```

اضافه شد.

default `cargo check` پس از اصلاح dependencyها و exportها موفق شد.

---

## 13. تغییرات `rustland-scheduler`

- `src/bpf_intf.rs` با نسخه exact upstream هماهنگ شد.
- dependencies زیر اضافه شدند:

```toml
plain = "=0.2.3"
procfs = "=0.18.0"
libc.workspace = true
ctrlc = { workspace = true, features = ["termination"] }
scx_utils = "=1.1.2"
```

- import اشتباه زیر حذف شد:

```rust
use scx_rustland_core::{DispatchedTask, QueuedTask};
```

- `BpfScheduler::init(...)` با API جدید هماهنگ شد:

```rust
let bpf = BpfScheduler::init(
    open_object,
    None,
    args.exit_dump_len,
    args.partial,
    args.debug,
    false,
    false,
    1_000_000,
    "afs_rustland",
)?;
```

- در `main.bpf.c` direct field writes با helperهای kernel جدید جایگزین شدند:

```text
prev->scx.slice = slice_ns;
→ scx_bpf_task_set_slice(prev, slice_ns);

p->scx.dsq_vtime = 0;
→ scx_bpf_task_set_dsq_vtime(p, 0);

p->scx.slice = slice_ns;
→ scx_bpf_task_set_slice(p, slice_ns);
```

- `bpf_skel` include shim از `OUT_DIR` است و نباید skeleton upstream روی آن کپی شود.

---

## 14. struct_ops attach EINVAL

خطا:

```text
Failed to attach struct_ops BPF programs
EINVAL
```

علت نهایی:

```text
afs-rustland
```

به‌عنوان scheduler name مشکل‌ساز بود.

نام صحیح تست‌شده:

```text
afs_rustland
```

Validation:

```text
STATE = enabled
OPS = afs_rustland_0.1.0_aarch64_unknown_linux_gnu
AFTER = disabled
EXIT: unregistered from user space
```

`builtin_idle=false` نیز configuration تست‌شده و موفق است.

`timeout` exit code `124` در runهای زمان‌دار به‌تنهایی crash محسوب نمی‌شود؛ sched_ext پس از stop باید به `disabled` برگردد.

---

## 15. Smoke configuration

`configs/smoke.toml`:

- cpus = 2
- repetitions = 1
- registry = `/tmp/ai-fusion-scheduler/registry`
- max_concurrent = 8
- `alpha_ms = 50`
- `beta_ms = 150`
- aging horizon = 500 ms
- batch base laxity = 1000 ms
- quantum = 2 ms
- critical slice = 2 ms
- interactive slice = 4 ms
- batch slice = 8 ms
- application deadline = true
- aging = true
- priority = true
- EWMA alpha = 0.25
- Admission safety factor = 1.10

Workload:

- 2 Critical CPU
- 3 Interactive CPU
- 2 Batch CPU

---

## 16. Dry-run

Dry-run موفق:

```text
tasks=7
completed=7
rejected=0
failed=0
misses=0
```

فایل‌های تولیدشده:

```text
run-input.json
runtime-estimates.json
summary.json
task-manifest.json
tasks.csv
```

**Dry-run نتیجه kernel scheduling نیست.**

---

## 17. Registry permission fix

خطا:

```text
Permission denied
```

برای فایل‌های registry رخ داد چون directory root-owned بود.

Fix:

```bash
sudo rm -rf /tmp/ai-fusion-scheduler
mkdir -p /tmp/ai-fusion-scheduler/registry
chmod 755 /tmp/ai-fusion-scheduler
chmod 755 /tmp/ai-fusion-scheduler/registry
```

از این پس:

```bash
rm -f /tmp/ai-fusion-scheduler/registry/*
```

و registry معمولی با `sudo mkdir` ساخته نشود.

---

## 18. اولین Real End-to-End با Admission v1

Generator:

```text
tasks=7
completed=6
rejected=1
failed=0
misses=0
```

Summary:

```json
{
  "total_tasks": 7,
  "admitted_immediately": 6,
  "rejected": 1,
  "completed": 6,
  "failed": 0,
  "deadline_misses": 0
}
```

Scheduler stats کلیدی:

```text
metadata_cache_hits = 18
metadata_registry_reads = 6
metadata_misses = 0
dispatch_errors = 0
user_dispatches = 24
failed_dispatches = 0
congestion_events = 0
scheduler_cpu_time_ns = 2199082
```

این run ثابت کرد مسیر واقعی زیر کار می‌کند:

```text
real worker
→ metadata registry
→ AFS userspace policy
→ sched_ext dispatch
```

---

## 19. False Rejection در Admission v1

Task:

```text
critical-cpu-2
```

نتیجه:

```text
REJECT
reason = predicted_infeasible
```

مشخصات تقریبی:

```text
estimated runtime ≈ 24.75 ms
relative deadline = 120 ms
priority = 1.0
class = Critical
```

---

## 20. Diagnostic run با Admission OFF

با `--no-admission`:

```text
total_tasks=7
rejected=0
completed=7
failed=0
deadline_misses=0
```

`critical-cpu-2`:

```text
actual_cpu_time_ns = 25,007,792
response_time_ns = 30,460,918
deadline_missed = false
```

یعنی:

```text
response ≈ 30.46 ms
deadline = 120 ms
```

پس برای همین Smoke workload، Admission v1 یک False Rejection قابل مشاهده داشت.

---

## 21. پیاده‌سازی Admission v2

### `policy-core/scoring.rs`

helper مشترک اضافه شد:

```text
relevant_interference_ns(...)
```

### `policy-core/admission.rs`

فرمول از تقریب:

```text
(active + own) / cpus
```

به:

```text
own + relevant_active / cpus
```

تغییر کرد.

### Real workload generator

generator برای active processها relevant interference را با policy مشترک محاسبه می‌کند.

### Simulator

simulator نیز همان helper را استفاده می‌کند تا semantics Admission بین simulator و اجرای واقعی متفاوت نشود.

---

## 22. Unit tests بعد از Admission v2

```text
10 passed
0 failed
```

تست‌های کلیدی:

```text
rejects_infeasible_deadline
critical_is_not_psi_rejected
own_runtime_is_not_parallelized_across_cpus
low_priority_batch_does_not_interfere_with_urgent_critical
lower_laxity_wins
aging_improves_waiting_task
virtual_deadline_is_not_mixed_with_monotonic_time
remaining_saturates
ewma_updates
```

`cargo check` برای workload-generator و policy-simulator نیز موفق شد.

---

## 23. Validation واقعی Admission v2

Summary:

```json
{
  "total_tasks": 7,
  "admitted_immediately": 7,
  "rejected": 0,
  "completed": 7,
  "failed": 0,
  "deadline_misses": 0
}
```

Critical 1:

```text
actual CPU ≈ 24.017 ms
response ≈ 33.008 ms
miss=false
```

Critical 2:

```text
estimated runtime ≈ 24.754 ms
actual CPU ≈ 25.001 ms
response ≈ 31.539 ms
deadline = 120 ms
miss=false
```

Interactive:

```text
~46.525 ms
~41.858 ms
~52.744 ms
```

Batch:

```text
~107.917 ms
~100.259 ms
```

Scheduler stats:

```text
metadata_cache_hits = 17
metadata_registry_reads = 7
metadata_misses = 0
dispatch_errors = 0
user_dispatches = 24
failed_dispatches = 0
congestion_events = 0
scheduler_cpu_time_ns = 1974085
```

---

## 24. زنجیره شواهد Admission

```text
Admission v1
critical-cpu-2
→ REJECT
→ predicted_infeasible

Admission OFF
critical-cpu-2
→ ADMIT
→ response ≈ 30.46 ms
→ deadline = 120 ms
→ deadline met

Admission v2
critical-cpu-2
→ ADMIT
→ response ≈ 31.54 ms
→ deadline met
```

نتیجه محدود و صحیح:

> برای این Smoke workload، Admission v1 یک False Rejection نشان داد و Admission v2 رفتار شناسایی‌شده را اصلاح کرد.

هنوز نباید ادعا شود Admission v2 universally superior یا optimal است.

---

## 25. داده‌هایی که فعلاً نباید پاک شوند

```text
results/smoke-real
results/smoke-no-admission
results/smoke-admission-v2
```

نام دقیق اولین real run باید در reproducibility snapshot با inventory نتایج ثبت شود.

---

## 26. Baselineها

1. Linux EEVDF
2. Stock `scx_rustland`
3. Proposed AFS
4. `SCHED_DEADLINE` فقط برای workloadهای periodic/sporadic که مقایسه منصفانه دارند

---

## 27. Ablations

1. no-aging
2. no-admission
3. no-application-deadline / virtual deadline

---

## 28. Metrics نهایی

- deadline miss ratio
- mean latency
- P95 latency
- throughput
- CPU utilization
- Jain fairness
- scheduler CPU time
- user/kernel/cancelled/bounced/failed dispatches
- congestion events
- notify-cycle metrics
- metadata cache behavior
- admission outcomes

هدف فعلی:

```text
10 repetitions
```

برای configurationهای نهایی.

---

## 29. محدودیت VM

VM فعلی برای development/correctness/integration مناسب است.

برای نتایج performance:

- virtualization disclosure لازم است.
- timing noise باید در limitations مطرح شود.
- Intel RAPL روی Mac M1/UTM مبنای مناسبی برای energy measurement نیست.
- bare-metal در صورت امکان برای benchmark نهایی ارجح است.

---

## 30. Massive Data Fusion case study

مسیر مفهومی:

```text
Data sources
→ ingestion
→ preprocessing
→ fusion
→ forecast/anomaly/decision
→ downstream action
```

Workflow/stage/deadline/priority می‌توانند metadata scheduling را تغذیه کنند.

این use case هنوز باید به workload نهایی reproducible تبدیل شود.

---

## 31. برنامه فیلم‌برداری استاد

فایل هدف:

```text
scripts/demo_for_professor.sh
```

Stages پیشنهادی:

```text
[1/6] Environment
[2/6] Loading AFS scheduler
[3/6] sched_ext status
[4/6] Running AI workload
[5/6] Results
[6/6] Unloading scheduler
```

در ویدئو:

- `uname -r`
- initial `disabled`
- config
- scheduler `enabled`
- ops name
- workload metadata/classes/deadline/priority
- summary metrics
- scheduler stats منتخب
- unload
- final `disabled`

۱۰ repetition در ویدئو لازم نیست؛ یک representative run + aggregate results کافی است.

---

## 32. فایل‌های مرجع

```text
docs/PROJECT_TRANSFER_REPORT.md
DECISIONS.md
ENVIRONMENT.md
CHANGELOG.md
PROJECT_CHECKLIST.md
```

پیشنهاد checkpoint machine-readable:

```text
docs/checkpoints/2026-08-09/
```

---

## 33. Freeze / snapshot مورد نیاز

قبل از benchmark اصلی ثبت شود:

```text
git status
git diff
git diff --stat
git rev-parse HEAD
Cargo.lock hash
important source hashes
release binary hashes
kernel version
OS release
rustc/cargo version
clang version
bpftool version
libbpf version
cmake/ninja version
Python version
scx commit
configs used
```

اسکریپت همراه bundle:

```text
scripts/capture_reproducibility_checkpoint.sh
```

---

## 34. Git checkpoint پیشنهادی

پس از review:

```bash
git add   Cargo.toml   Cargo.lock   rust-toolchain.toml   crates   configs   docs   DECISIONS.md   ENVIRONMENT.md   CHANGELOG.md   PROJECT_CHECKLIST.md   scripts

git diff --cached

git commit -m "checkpoint: working sched_ext AFS with admission v2"
```

---

## 35. ادامه کار پیشنهادی

### A — بستن Checkpoint

- [x] sched_ext kernel
- [x] stock RustLand
- [x] AFS attach
- [x] real metadata path
- [x] real workload
- [x] Admission v1 false rejection reproduction
- [x] Admission v2
- [x] unit tests
- [x] real v2 Smoke
- [ ] capture reproducibility snapshot
- [ ] review Git diff
- [ ] checkpoint commit/tag

### B — inventory

```bash
find configs -maxdepth 2 -type f -print | sort
find scripts -maxdepth 2 -type f -print | sort
```

### C — benchmark harness

- deterministic manifest reuse
- fixed configs
- output naming
- 10 repetitions
- baseline lifecycle verification
- failure handling
- integrity checks

### D — baseline / ablation / workload suite

سپس aggregation، plots، tables، proposal/report/paper/slides و demo.

---

## 36. نکات فراموش‌نشدنی

1. kernel 6.12.95 sched_ext مناسب نداشت.
2. kernel فعلی 7.1.3+deb13-arm64 است.
3. scx commit باید freeze بماند.
4. `afs-rustland` attach EINVAL داد؛ `afs_rustland` کار کرد.
5. `builtin_idle=false` تست‌شده است.
6. registry نباید root-owned شود.
7. timeout 124 لزوماً scheduler failure نیست.
8. Dry-run با real kernel experiment یکی نیست.
9. Smoke results نتیجه نهایی مقاله نیست.
10. Admission v1 false rejection واقعاً مشاهده شد.
11. Admission v2 هنوز stress validation گسترده لازم دارد.
12. simulator و real generator باید Admission semantics مشترک داشته باشند.
13. VM باید در methodology/limitations گزارش شود.
14. debug history از demo نهایی جدا باشد.
15. هر benchmark به commit/config/manifest/repetition قابل ردیابی باشد.

---

## 37. Definition of Done این مرحله

```text
[ ] docs checkpoint applied
[ ] environment snapshot captured
[ ] git diff reviewed
[ ] tested code committed/tagged
[ ] smoke diagnostic outputs preserved
[ ] configs/scripts inventory captured
```

پس از آن پروژه وارد فاز **Benchmark Engineering** می‌شود.

---

## 38. خلاصه مدیریتی

تا 2026-08-09 پروژه از مرحله طراحی صرف عبور کرده و یک scheduler واقعی مبتنی بر `sched_ext` روی Linux ARM64 VM اجرا شده است.

Stock RustLand و AFS هر دو روی kernel `7.1.3+deb13-arm64` attach/detach موفق داشته‌اند. مسیر metadata واقعی فعال است، workload واقعی اجرا شده و userspace dispatch ثبت شده است.

در اولین Smoke End-to-End، Admission v1 یک Critical task را با `predicted_infeasible` رد کرد. آزمایش Admission OFF نشان داد همان task با response حدود `30.46 ms` deadline `120 ms` را رعایت می‌کند؛ بنابراین برای آن workload یک False Rejection وجود داشت.

Admission v2 با دو اصلاح اصلی ساخته شد:

1. runtime خود task بین CPUها تقسیم نمی‌شود.
2. interference بر اساس ranking واقعی policy محاسبه می‌شود.

Unit testهای policy-core با `10/10` موفق شدند. Real Smoke با Admission v2 نیز `7/7 completion`, `0 rejection`, `0 failure`, `0 deadline miss` داشت. Critical task دوم با response حدود `31.54 ms` پذیرفته شد و deadline را رعایت کرد.

مرحله بعد: reproducibility freeze → benchmark harness → baseline/ablation → 10 repetitions → aggregation → update academic artifacts → professor demo.

---

## 39. تاریخچه Checkpointها

### Checkpoint 1 — 2026-08-09

**نام:** sched_ext + AFS end-to-end + Admission v2 validation
**وضعیت:** آماده برای freeze و ورود به Benchmark Engineering
