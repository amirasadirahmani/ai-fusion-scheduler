# به روزرسانی روش شناسی پروپوزال - AI Fusion Scheduler

> این سند نسخه checkpoint روش شناسی است و جایگزین نتیجه نهایی پروپوزال نیست. هدف آن ثبت معماری و پروتکل آزمایش بعد از checkpoint `checkpoint-paper-harness-v1` است.

## عنوان پیشنهادی

طراحی و ارزیابی یک زمان بند آگاه از کاربرد برای بارهای کاری هوش مصنوعی با استفاده از Linux sched_ext و کنترل پذیرش مبتنی بر فشار workload

## مسئله پژوهش

بارهای کاری هوش مصنوعی در محیط های edge/cloud معمولاً ترکیبی از وظایف critical، interactive و batch هستند. این وظایف deadline، priority و حساسیت متفاوت به latency دارند. زمان بندهای عمومی سیستم عامل معمولاً از metadata سطح کاربرد مثل deadline نرم، class workload، و priority کاربردی استفاده مستقیم نمی کنند.

هدف پروژه طراحی و ارزیابی یک scheduler تجربی است که بتواند با استفاده از `sched_ext`، metadata کاربردی و admission control، رفتار deadline-aware بهتری نسبت به baselineها ایجاد کند.

## اصلاحات مهم روش شناسی

در مسیر پیاده سازی چند نکته مهم مشخص شد که در پروپوزال باید ثبت شوند:

1. **CPU واقعی آزمایش باید با مدل admission یکی باشد.** مقدار `experiment.cpus` اکنون به صورت affinity واقعی اعمال می شود و فقط یک پارامتر محاسباتی نیست.
2. **ساعت release workload باید بعد از initialization شروع شود.** در غیر این صورت latency مربوط به فایل سیستم و sync ممکن است deadline را قبل از شروع workload مصرف کند.
3. **PSI کل سیستم برای admission قابل اتکا نیست.** در محیط userspace sched_ext، حتی بدون workload واقعی، system-wide PSI می تواند به علت control-plane بالا برود. بنابراین admission از `cpu.pressure` مربوط به cgroup مخصوص workload استفاده می کند.

## معماری فعلی

```text
Control CPUs
  - userspace scheduler
  - workload generator
  - experiment harness

Workload CPUs
  - critical workers
  - interactive workers
  - batch workers
  - workload cgroup cpu.pressure
```

تمام workerهای workload داخل cgroup اختصاصی هر run قرار می گیرند. scheduler و generator بیرون این cgroup باقی می مانند. بنابراین سیگنال فشار CPU که برای admission استفاده می شود فقط مربوط به workload کاربردی است.

## روش ارزیابی پیشنهادی

روش ها:

- Linux EEVDF
- Stock RustLand
- AFS بدون admission
- AFS کامل با admission
- Ablationها در صورت زمان کافی

پروفایل های workload:

- Light: ظرفیت کافی، انتظار rejection کم یا صفر
- Saturated: نزدیک ظرفیت، آشکار شدن تفاوت schedulerها
- Overload: فشار واقعی، ارزیابی load shedding و deadline goodput

## معیارهای اصلی

معیار اصلی نباید فقط deadline miss بین taskهای پذیرفته شده باشد. چون admission می تواند با reject کردن taskها miss rate را مصنوعی کم کند.

معیار اصلی:

```text
Deadline Goodput = تعداد taskهای deadlineدار که قبل از deadline تمام شدند / کل taskهای deadlineدار ارائه شده
```

معیارهای تکمیلی:

- rejection rate
- completion rate
- accepted-task miss rate
- P50/P95 response time
- throughput
- CPU utilization
- Jain fairness
- scheduler overhead
- workload PSI

## وضعیت فعلی

پروژه اکنون به یک checkpoint مهندسی معتبر رسیده است. زیرساخت آزمایش، CPU isolation، release clock، workload-scoped PSI، validation و provenance آماده شده اند. هنوز workloadهای نهایی مقاله و نتایج نهایی freeze نشده اند.

## گام بعدی

1. تعریف traceهای sustained برای light/saturated/overload
2. اجرای pilot سه تکراری
3. رفع فقط اشکالات روش شناختی واقعی
4. freeze کد و config
5. اجرای benchmark نهایی و استخراج نمودارها
6. آماده سازی گزارش، ارائه، فیلم استاد و مقاله کنفرانسی
