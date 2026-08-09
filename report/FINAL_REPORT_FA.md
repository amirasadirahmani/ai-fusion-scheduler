# گزارش نهایی پروژه

**عنوان:** طراحی و ارزیابی یک زمان بند Soft-Deadline آگاه از کاربرد برای بارهای کاری هوش مصنوعی در لینوکس با استفاده از Rust و sched_ext

> وضعیت: این فایل قالب گزارش نهایی است و پس از اجرای آزمایش های واقعی باید با نتایج واقعی تکمیل شود. اعداد موجود در پوشه `results/sample` صرفا نمونه ساختگی برای تست ابزار تحلیل هستند.

## چکیده

این پروژه یک سیاست زمان بندی Soft-Deadline آگاه از کاربرد را برای بارهای کاری ناهمگن AI و Massive Data Fusion طراحی و ارزیابی می کند. سیاست پیشنهادی با Rust در فضای کاربر پیاده سازی شده و از طریق `scx_rustland_core` و `sched_ext` به زمان بندی CPU در لینوکس متصل می شود. هدف اصلی، بررسی این پرسش است که آیا Deadline صریح سطح کاربرد، تخمین زمان اجرای باقی مانده، کلاس وظیفه، Aging و Admission Control می توانند نسبت به EEVDF، `SCHED_DEADLINE` و `scx_rustland` استاندارد، نرخ Deadline Miss و Tail Latency را کاهش دهند یا خیر.

## 1. مقدمه

بارهای کاری AI و Data Fusion از مراحل ناهمگن تشکیل می شوند. برخی مراحل مانند پاسخ تعاملی یا تولید هشدار دارای Deadline کوتاه هستند و برخی مراحل مانند Indexing یا پردازش دسته ای می توانند با اولویت پایین تر اجرا شوند. زمان بند عمومی لینوکس از Deadline واقعی کاربرد و جایگاه هر مرحله در Pipeline آگاه نیست. این پروژه تلاش می کند این اطلاعات را به سیاست زمان بندی منتقل کند.

## 2. هدف و سوال های پژوهش

سوال اصلی:

> آیا استفاده از Deadline صریح سطح کاربرد و تخمین زمان اجرای باقی مانده، در مقایسه با Deadlineهای مجازی و استنتاج شده در زمان بندهای موجود، نرخ از دست رفتن مهلت بارهای AI و Data Fusion را کاهش می دهد؟

سوال های فرعی:

1. سربار اجرای سیاست Rust و ارتباط با BPF چقدر است؟
2. Aging چه تاثیری بر عدالت و جلوگیری از Starvation دارد؟
3. Admission Control در شرایط Overload چه اثری بر Deadline Miss و Tail Latency دارد؟
4. روش پیشنهادی در چه سناریوهایی نسبت به `SCHED_DEADLINE` یا `scx_rustland` مناسب تر است؟

## 3. معماری پیاده سازی

معماری پروژه:

```text
Config-driven Rust Workload Generator
        ↓
Rust Metadata Manager
        ↓
Rust Scheduling and Admission Policy
Application Deadline + Laxity + Aging
        ↓
scx_rustland_core
        ↓
BPF Backend + Linux sched_ext
```

## 4. مدل فراداده

برای هر وظیفه فیلدهای زیر ثبت می شود:

| فیلد | توضیح |
|---|---|
| `task_id` | شناسه یکتای وظیفه |
| `process_id` | شناسه پردازه یا Thread |
| `workflow_id` | شناسه Pipeline یا جریان کاری |
| `stage_id` | شناسه مرحله در Pipeline |
| `workload_class` | یکی از `critical`، `interactive` یا `batch` |
| `absolute_deadline` | Deadline مطلق وظیفه |
| `estimated_remaining_runtime` | زمان اجرای باقی مانده تخمینی |
| `application_priority` | اولویت اعلام شده از سوی کاربرد |
| `release_time` | زمان ورود وظیفه |

## 5. سیاست زمان بندی

تخمین زمان باقی مانده:

```text
R_hat_i(t) = max(0, C_hat_i - C_i_consumed(t))
```

Laxity:

```text
L_i(t) = D_i - t - R_hat_i(t)
```

امتیاز نهایی:

```text
S_i(t) = L_i(t) - alpha * P_i_norm - beta * A_i_norm(t)
```

وظیفه ای که کمترین مقدار `S_i(t)` را داشته باشد در اولویت Dispatch قرار می گیرد.

## 6. کنترل پذیرش

کنترل پذیرش سه خروجی دارد:

- `ADMIT`: پذیرش وظیفه
- `DELAY`: تاخیر در ورود وظیفه
- `REJECT`: رد وظیفه

تصمیم بر اساس Deadline، تخمین Runtime، تعداد وظایف فعال و فشار CPU از PSI گرفته می شود.

## 7. Baselineها

| روش | نقش در ارزیابی |
|---|---|
| EEVDF | زمان بند Fair پیش فرض لینوکس |
| `SCHED_DEADLINE` | مقایسه در سناریوهای Periodic/Sporadic قابل پیکربندی |
| `scx_rustland` | نزدیک ترین Baseline مبتنی بر Deadline مجازی |
| روش پیشنهادی | Application Deadline + Laxity + Aging + Admission Control |

## 8. سناریوهای ارزیابی

- Smoke: بررسی صحت اجرا
- Light: بار کمتر از ظرفیت
- Saturated: بار نزدیک ظرفیت
- Overload/Bursty: بار بیشتر از ظرفیت
- Periodic: مناسب برای مقایسه با `SCHED_DEADLINE`
- Fusion Pipeline: مطالعه موردی Massive Data Fusion

## 9. معیارهای ارزیابی

- Deadline Miss Ratio
- P95 Response Time
- Throughput
- Goodput
- CPU Utilization
- Jain's Fairness Index
- Rejection Rate
- Scheduler CPU Fraction
- User-space Dispatches
- Kernel Dispatches
- Failed/Bounced/Cancelled Dispatches
- Average Tasks per `notify_complete` cycle

## 10. نتایج آزمایش ها

### 10.1 خلاصه نتایج اصلی

> این بخش پس از اجرای واقعی روی Linux 6.12.95 تکمیل شود.

| سناریو | روش | Deadline Miss | P95 Response | Goodput | Fairness |
|---|---|---:|---:|---:|---:|
| TODO | TODO | TODO | TODO | TODO | TODO |

### 10.2 تحلیل Deadline Miss

TODO: نمودار `deadline-miss-ratio.png` را از `results/summary/plots` وارد و تحلیل کن.

### 10.3 تحلیل Tail Latency

TODO: نمودار `p95-response.png` را وارد و توضیح بده کدام روش در بارهای Overload بهتر عمل کرده است.

### 10.4 تحلیل Fairness

TODO: بررسی کن Aging چه تاثیری بر سرویس دهی به کلاس Batch دارد.

### 10.5 سربار Scheduler

TODO: آمار `scheduler-stats.json` و ستون های dispatch را تحلیل کن.

## 11. Ablation Study

| Ablation | هدف |
|---|---|
| بدون Aging | سنجش تاثیر Aging بر Fairness و Starvation |
| بدون Admission Control | سنجش تاثیر کنترل پذیرش بر Overload |
| بدون Application Deadline | مقایسه با Virtual Deadline مشابه Rustland |

## 12. مطالعه موردی Massive Data Fusion

Pipeline موردی شامل مراحل زیر است:

1. Ingestion
2. Preprocessing
3. Feature Extraction
4. Fusion
5. Decision/Alert
6. Archival/Indexing

هدف این Case Study بررسی Scheduler است، نه طراحی الگوریتم Fusion جدید.

## 13. تهدیدهای اعتبار

- اجرای آزمایش روی VM می تواند نتایج CPU Scheduling را مخدوش کند.
- تخمین Runtime ساده است و خطای تخمین روی Laxity تاثیر می گذارد.
- `sched_ext` و APIهای آن ممکن است بین نسخه های کرنل تغییر کنند.
- نتایج Simulator جایگزین نتایج واقعی کرنل نیستند.

## 14. نتیجه گیری

TODO: پس از اجرای آزمایش ها، پاسخ مستقیم به سوال اصلی پژوهش را بنویس. اگر روش پیشنهادی فقط در Overload بهتر بود، همان را صادقانه گزارش کن. اگر `SCHED_DEADLINE` در بارهای Periodic بهتر بود، آن را به عنوان نتیجه معتبر بنویس.

## 15. کارهای آینده

- انتقال بخشی از Hot Path به BPF
- مقایسه کامل با `scx_lavd`
- افزودن GPU Runtime Telemetry
- ادغام با Kubernetes یا Edge/Cloud Orchestrator
- تخمین Runtime با مدل آماری پیشرفته تر

## پیوست A: دستورهای بازتولید

```bash
./scripts/check_environment.sh
./scripts/collect_system_info.sh results/environment-before.txt
./scripts/build_userspace.sh
./scripts/build_sched_ext.sh
sudo ./scripts/run_smoke_test.sh
./scripts/run_simulator_matrix.sh
./scripts/run_ablations.sh
./scripts/run_fusion_case.sh
python analysis/analyze_results.py results/**/tasks.csv --out results/summary
python analysis/plot_results.py results/summary/aggregate-summary.csv --out-dir results/summary/plots
```
