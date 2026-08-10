# Architecture decision record

## ADR-001 - User-space policy on rustland_core

**Decision:** implement the research policy in Rust user space and use `scx_rustland_core` as the BPF bridge.

**Reason:** rapid policy iteration, ordinary Rust data structures, simpler logging and testability. The user/kernel communication cost is explicitly measured rather than hidden.

**Alternative:** move the hot path to BPF, as in LAVD/bpfland. Deferred to future work if measured overhead is too high.

## ADR-002 - Partial sched_ext mode

**Decision:** handle only tasks explicitly set to `SCHED_EXT`.

**Reason:** preserve EEVDF for the shell, SSH and system daemons during development. This is safer and makes the experiment population explicit.

## ADR-003 - Runtime estimation

**Decision:** use per-task calibrated total CPU-time estimates, subtract observed consumed CPU time, and optionally update class estimates with EWMA.

**Reason:** sufficient for a one-semester prototype; machine-learning prediction is out of scope.

## ADR-004 - CPU PSI only

**Decision:** admission control uses `/proc/pressure/cpu`, primarily `some avg10`.

**Reason:** memory and I/O pressure control would substantially expand the parameter space. They remain measured workload characteristics but are not admission signals in the required version.

## ADR-005 - Fusion case study scope

**Decision:** implement a lightweight pipeline with heterogeneous stages, not a novel fusion algorithm.

**Reason:** the research object is node-level operating-system scheduling for fusion workers.

<!-- CHECKPOINT-2026-08-09:START -->
## Checkpoint 2026-08-09 — تصمیم‌های تثبیت‌شده

1. Kernel توسعه روی `7.1.3+deb13-arm64` تثبیت شد؛ kernel اولیه `6.12.95` فاقد `CONFIG_SCHED_CLASS_EXT` بود.
2. scx commit این مرحله: `7a58a3a1a857fa748ea5a3bda5ad312afbbeca5b`.
3. scheduler name تست‌شده: `afs_rustland`؛ فرم `afs-rustland` باعث attach `EINVAL` شد.
4. configuration تست‌شده شامل `builtin_idle=false`, `numa_local=false`, minimum slice = `1_000_000 ns` است.
5. ranking اصلی: `score = laxity - priority_bonus - aging_bonus` و score کمتر برنده است.
6. Admission نباید با `Critical => always admit` پیاده شود.
7. Admission v2 از `own_runtime + relevant_interference/cpus` استفاده می‌کند.
8. relevant interference با ranking اصلی policy هم‌راستا است.
9. generator واقعی و simulator باید semantics مشترک Admission داشته باشند.
10. Dry-run، Smoke و Diagnostic runها نتایج نهایی benchmark نیستند.
11. Baselineها: EEVDF، stock RustLand، Proposed AFS و SCHED_DEADLINE فقط در workload منصفانه.
12. Ablationها: no-aging، no-admission، no-application-deadline.
13. benchmark نهایی باید به config/manifest/repetition/commit قابل ردیابی باشد و هدف فعلی 10 repetition است.
14. virtualization باید در methodology/limitations گزارش شود.
15. debug runها از demo نهایی استاد جدا می‌شوند.
16. passwordless sudo فقط برای VM آزمایشگاهی پذیرفته شده است.

### تصمیم طراحی Admission

Admission v1، `critical-cpu-2` را با `predicted_infeasible` رد کرد. همان task با Admission خاموش در حدود `30.46 ms` پاسخ داد و deadline `120 ms` را رعایت کرد.

Admission v2 همان task را پذیرفت و response حدود `31.54 ms` با `deadline_missed=false` ثبت شد.

این شاهد برای اصلاح طراحی معتبر است، اما هنوز ادعای برتری عمومی Admission v2 نیست.
<!-- CHECKPOINT-2026-08-09:END -->

## Paper benchmark engineering decisions — 2026-08-10

### Enforce the configured CPU count physically

`experiment.cpus` must correspond to the CPU resources available to workload
workers, not merely to a value used by admission calculations. The harness
therefore partitions CPUs into workload and control-plane sets and enforces
worker affinity.

### Start the workload clock after initialization

Potentially slow initialization and durable output I/O must finish before the
workload release origin is established. Infrastructure latency must not
consume an application's soft-deadline budget before its release.

### Use workload-scoped CPU PSI

Admission decisions use the CPU PSI of a dedicated cgroup containing only
application workers.

System-wide `/proc/pressure/cpu` was rejected as the admission-pressure
source because idle experiments demonstrated substantial PSI when both the
proposed userspace sched_ext scheduler and stock RustLand were active without
application workload.

### Preserve offered-load semantics

Rejected tasks remain part of the offered workload. Publication evaluation
must therefore report offered-task deadline goodput, acceptance/rejection
rate, and completion rate rather than comparing deadline misses only among
accepted tasks.

### Separate scheduling from admission effects

Paper evaluation must include AFS without admission in addition to full AFS.
This separates improvements caused by scheduling policy from improvements
caused by admission/load shedding.
