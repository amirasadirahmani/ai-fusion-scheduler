# گزارش نهایی پروژه AI Fusion Scheduler (AFS)

## چکیده

AI Fusion Scheduler (AFS) یک زمان بند آزمایشی برای Linux است که با هدف مدیریت بارهای ناهمگن، deadline-aware و فشار بالا طراحی شده است. مسئله اصلی پروژه این است که یک scheduler عمومی در شرایطی که taskهای critical، interactive، batch و workloadهای ناهمگن هم زمان فعال هستند، لزوما اطلاعات کاربردی مانند deadline، priority و laxity را به شکلی که برای کیفیت سرویس انتها به انتها مناسب باشد در اختیار ندارد. AFS این شکاف را با ترکیب policy scoring، metadata کاربردی، admission control و پیاده سازی مبتنی بر `sched_ext` هدف قرار می دهد.

ارزیابی نهایی پروژه از یک قرارداد frozen استفاده می کند. ماتریس اصلی شامل 120 اجرای معتبر است: 3 workload در 4 روش و 10 تکرار. روش ها شامل EEVDF، RustLand اصلی، AFS بدون admission و AFS کامل هستند. ارزیابی ثانویه نیز شامل 20 اجرای no-aging، 20 اجرای no-application-deadline، 30 اجرای اندازه گیری شده periodic/sporadic به همراه 10 اجرای manifest-seed، و یک مطالعه موردی Fusion است. تمام 80 validation استاندارد ارزیابی ثانویه با وضعیت OK پایان یافته اند.

نتایج نشان می دهند که در بار سبک، همه روش ها در ناحیه نزدیک به سقف 100 درصد قرار دارند و AFS نباید در این regime به عنوان برنده قطعی معرفی شود. در بار saturated، AFS کامل goodput معادل `82.17 ± 3.08%` ثبت می کند که از نظر confidence interval با RustLand (`83.57 ± 13.06%`) هم پوشانی دارد؛ با این حال AFS accepted miss، P95 و فشار workload را به شکل محسوسی کاهش می دهد، به قیمت `18.05 ± 2.34%` rejection. در overload، admission اثر اصلی خود را نشان می دهد: AFS goodput را به `13.32 ± 1.58%` می رساند و P95 را به حدود `7.03 s` محدود می کند، اما `79.38 ± 0.45%` از بار را رد می کند. بنابراین نتیجه overload یک trade-off صریح load shedding است، نه یک بهبود بدون هزینه.

Ablationها نشان می دهند application-deadline awareness نقش مهمی در کیفیت سیاست دارد و حذف آن goodput را در saturated به `58.70%` و در overload به `0.40%` کاهش می دهد. حذف aging نیز در overload accepted miss را به `51.24%` افزایش می دهد. در workload periodic/sporadic، AFS goodput معادل `99.87%`، EEVDF برابر `99.33%` و SCHED_DEADLINE برابر `96.53%` ثبت کرده اند؛ این مقایسه فقط برای workload frozen همین آزمایش معتبر است. مطالعه موردی Fusion نیز با 120 task، 45 completed، 75 rejected، 0 failed و 19 deadline miss با موفقیت اجرا شده است.

جمع بندی پروژه این است که AFS در شرایط فشار بالا، با استفاده از deadline-aware policy و admission control می تواند کیفیت کار پذیرفته شده را کنترل کند، اما rejection بخش جدایی ناپذیر این رفتار است. ادعاهای نهایی پروژه بر offered deadline goodput همراه با rejection، completion و accepted miss متکی هستند و از تعمیم pilot، diagnostic یا شمارنده های داخلی overhead به ادعاهای عمومی خودداری می شود.

---

## 1. مقدمه

سیستم های مدرن معمولا ترکیبی از workloadهای تعاملی، محاسباتی، حافظه محور و deadline-sensitive را روی یک ماشین اجرا می کنند. در چنین محیطی یک معیار واحد مانند throughput یا CPU utilization برای توصیف کیفیت scheduler کافی نیست. ممکن است سیستم از نظر throughput فعال باشد، اما taskهای دارای deadline دیر تکمیل شوند؛ یا ممکن است latency پایین باشد اما تنها به دلیل drop یا rejection گسترده بار.

هدف AFS طراحی و ارزیابی یک scheduler آزمایشی است که بتواند metadata کاربردی را در تصمیم زمان بندی دخالت دهد و در شرایط overload به جای اجازه دادن به رشد کنترل نشده صف، بخشی از بار را به صورت صریح رد یا defer کند. این پروژه بیش از آنکه ادعای جایگزینی عمومی scheduler لینوکس را داشته باشد، یک مطالعه مهندسی و تجربی درباره ترکیب سه ایده است:

1. deadline و laxity در policy scheduling؛
2. priority و aging برای کنترل starvation و ordering؛
3. admission control برای مهار overload.

پیاده سازی روی Linux `sched_ext` انجام شده است تا policy آزمایشی در فضای کاربر توسعه یابد، در حالی که مسیر scheduling از زیرساخت kernel بهره می برد.

---

## 2. مسئله پژوهش

مسئله اصلی این پروژه را می توان چنین بیان کرد:

> چگونه می توان در یک host لینوکسی با workloadهای ناهمگن، کیفیت deadline taskهای پذیرفته شده را در شرایط saturation و overload کنترل کرد، بدون آنکه نتایج با حذف خاموش بار یا متریک های ناقص خوش بینانه گزارش شوند؟

این مسئله چند دشواری هم زمان دارد:

- deadline taskها با batch taskها برای CPU رقابت می کنند؛
- priority به تنهایی می تواند starvation ایجاد کند؛
- deadline بدون توجه به runtime یا laxity می تواند ترتیب نامناسبی ایجاد کند؛
- overload پایدار باعث رشد latency و miss می شود؛
- admission می تواند کیفیت accepted work را بهتر کند، اما با rejection همراه است؛
- اندازه گیری scheduler در `sched_ext` می تواند با control plane و PSI سیستم تداخل داشته باشد؛
- مقایسه با SCHED_DEADLINE نیازمند root-domain و cpuset صحیح است.

AFS تلاش می کند این مشکلات را در یک طراحی منسجم حل و سپس با یک methodology frozen ارزیابی کند.

---

## 3. اهداف پروژه

اهداف نهایی پروژه عبارت اند از:

- پیاده سازی یک policy scheduler مبتنی بر deadline، laxity، priority و aging؛
- افزودن admission control آگاه از فشار و deadline؛
- جداسازی workload CPUها از control CPUها؛
- اندازه گیری workload-scoped CPU PSI به جای PSI سراسری آلوده به control plane؛
- مقایسه با EEVDF و RustLand در workloadهای light، saturated و overload؛
- جداسازی اثر admission از policy با AFS بدون admission؛
- اجرای ablation برای aging و application-deadline awareness؛
- اجرای مقایسه periodic/sporadic با Linux SCHED_DEADLINE؛
- اجرای یک مطالعه موردی end-to-end برای Fusion pipeline؛
- ارائه تحلیل آماری تکرارپذیر با confidence interval و provenance قابل بررسی.

---

## 4. پرسش های پژوهش

### RQ1 - رفتار در بار سبک

آیا AFS در شرایط light-load باعث افت محسوس نسبت به baselineها می شود؟

### RQ2 - کیفیت deadline در saturation

آیا AFS می تواند در saturation کیفیت deadline taskهای پذیرفته شده، tail latency و pressure را کنترل کند؟

### RQ3 - نقش admission در overload

آیا admission control می تواند از فروپاشی latency و miss در overload جلوگیری کند، و هزینه rejection آن چقدر است؟

### RQ4 - نقش اجزای policy

آیا aging و application-deadline awareness تاثیر قابل مشاهده ای در نتایج دارند؟

### RQ5 - رفتار periodic/sporadic

AFS در workload periodic/sporadic frozen در مقایسه با EEVDF و SCHED_DEADLINE چگونه عمل می کند؟

### RQ6 - امکان اجرای pipeline ناهمگن

آیا مسیر کامل AFS می تواند یک Fusion pipeline ناهمگن را end-to-end اجرا کند؟

---

## 5. معماری سیستم

AFS از چند جزء اصلی تشکیل شده است.

### 5.1 Metadata و workload description

برای taskها metadata شامل class، priority، deadline و اطلاعات مربوط به workload نگهداری می شود. policy از این metadata برای محاسبه score و تصمیم ordering استفاده می کند.

### 5.2 Policy scoring

نسخه frozen policy از ترکیب deadline laxity، batch base laxity، priority نرمال شده، aging و tie-break استفاده می کند. پارامترهای frozen اصلی:

- `alpha = 800 ms`
- `beta = 4000`
- `aging horizon = 16000`
- `batch base laxity = 40000`
- `quantum = 2 ms`
- priorityهای critical / interactive / batch به ترتیب 2 / 4 / 8
- application deadline: enabled
- aging: enabled
- priority: enabled
- runtime source: `/proc`
- EWMA runtime coefficient: `0.25`

هدف policy این است که taskهای deadline-sensitive را با توجه به زمان باقی مانده و runtime تخمینی جلو بیندازد، بدون آنکه batchها برای همیشه حذف شوند.

### 5.3 Admission control

نسخه frozen admission از منطق deadline-aware deferral استفاده می کند. پارامترهای اصلی:

- admission enabled: true
- PSI delay threshold: 20
- reject threshold: 60
- safety factor: 1.20
- max delay: 4000 ms
- delay step: 200 ms
- reject below priority: 0.80

Admission بخشی از workload را در فشار بالا defer یا reject می کند. بنابراین معیار goodput باید offered-load semantics را حفظ کند تا rejection باعث بهتر دیده شدن مصنوعی scheduler نشود.

### 5.4 CPU partitioning

روی VM شش CPU منطقی وجود دارد. تقسیم frozen:

- workload CPUs: `2,3,4,5`
- control CPUs: `0,1`

این تقسیم به صورت فیزیکی enforce شده است. هدف، جلوگیری از آلودگی workload measurements توسط scheduler control plane و ابزارهای orchestration است.

### 5.5 Workload-scoped PSI

استفاده از `/proc/pressure/cpu` در آزمایش های اولیه نشان داد که فعالیت `sched_ext` control plane می تواند PSI سراسری را آلوده کند. نسخه frozen یک cgroup v2 delegated به نام `afs-workload` می سازد و workerها را در آن قرار می دهد. Admission و تحلیل فشار از `cpu.pressure` همین cgroup استفاده می کنند.

### 5.6 RustLand و sched_ext

AFS روی RustLand و `sched_ext` ساخته شده است. revision frozen scx:

`7a58a3a1a857fa748ea5a3bda5ad312afbbeca5b`

Binary اصلی RustLand:

`/home/amir/scx/target/release/scx_rustland`

---

## 6. محیط آزمایش

محیط نهایی:

- Debian 13 ARM64
- Linux kernel: `7.1.3+deb13-arm64`
- VM روی Apple Silicon / UTM
- 6 CPU منطقی
- حدود 7.7 GiB RAM
- حدود 3.3 GiB swap
- cgroup v2
- `sched_ext` فعال و قابل استفاده
- workload CPUs: 2-5
- control CPUs: 0-1

برای جلوگیری از OOM در pilot، working set حافظه workload اصلاح شد:

- interactive memory: 8 MiB
- batch/fixed memory: 16 MiB

Pilot فقط برای qualification استفاده شده و هیچ ادعای final از pilot استخراج نشده است.

---

## 7. workloadهای اصلی

سه workload frozen برای ماتریس اصلی استفاده شده اند.

### 7.1 Light

- 67 task
- critical: 20 CPU
- interactive: 35 mixed
- memory per interactive task: 8 MiB
- batch: 12 CPU

هدف این workload بررسی عدم افت در شرایطی است که scheduler تحت فشار شدید نیست.

### 7.2 Saturated

- 133 task
- critical: 45
- interactive: 70 mixed
- memory per interactive task: 8 MiB
- batch: 18 memory
- batch memory: 16 MiB

این workload برای ایجاد فشار پایدار بدون ورود کامل به overload شدید طراحی شده است.

### 7.3 Overload

- 770 task
- critical: 240 با burst=8
- interactive: 480 با burst=12
- memory per interactive task: 8 MiB
- batch: 50 fixed-memory
- batch memory: 16 MiB

این workload برای آشکار کردن رفتار admission/load shedding استفاده شده است.

---

## 8. روش های مقایسه شده

چهار روش در ماتریس primary:

1. **EEVDF** - baseline scheduler لینوکس؛
2. **RustLand** - اجرای stock RustLand؛
3. **AFS no admission** - policy AFS بدون admission؛
4. **Full AFS** - policy کامل به همراه admission.

در periodic/sporadic سه روش مقایسه شده اند:

- EEVDF
- Full AFS
- SCHED_DEADLINE

---

## 9. قرارداد تحلیل آماری

قرارداد frozen analysis:

- `offered_deadline_tasks`
- `deadline_successes`
- `deadline_goodput`
- `accepted_deadline_tasks`
- `accepted_deadline_misses`
- `accepted_miss_rate`
- `rejection_rate`
- `completion_rate`

task deadlineداری که reject می شود همچنان در مخرج offered deadline goodput باقی می ماند.

همچنین:

- P50 و P95 response time
- breakdown در سطح class
- workload-scoped CPU PSI
- Jain class-service fairness به عنوان secondary metric
- overhead داخلی AFS فقط برای خود AFS
- repetition به عنوان statistical unit
- میانگین و two-sided Student-t 95% CI
- `n=10`
- `df=9`
- `t=2.262`

shared manifestها pairing بین روش ها را حفظ می کنند.

---

## 10. صحت و integrity ارزیابی

### 10.1 ماتریس اصلی

ماتریس final:

- 3 workload
- 4 method
- 10 repetition
- مجموع: 120 run

وضعیت:

- 120/120 validation OK
- 30 manifest
- hash integrity: OK
- OOM: صفر
- final sched_ext state: disabled

### 10.2 ارزیابی ثانویه

- no-aging: 20/20
- no-application-deadline: 20/20
- periodic measured: 30/30
- periodic seed: 10/10
- standard validations: 80/80 OK
- fusion: complete
- publication plots: 32 PNG
- final sched_ext state: disabled

---

## 11. نتایج اصلی

### 11.1 Light workload

| Method | Goodput % | Accepted miss % | Rejection % | Completion % | P50 ms | P95 ms | PSI avg10 |
|---|---:|---:|---:|---:|---:|---:|---:|
| EEVDF | 100.00 ± 0.00 | 0.00 ± 0.00 | 0.00 ± 0.00 | 100.00 ± 0.00 | 568.6 ± 17.4 | 2447.0 ± 46.6 | 0.36 ± 0.26 |
| RustLand | 100.00 ± 0.00 | 0.00 ± 0.00 | 0.00 ± 0.00 | 100.00 ± 0.00 | 567.3 ± 19.9 | 2470.2 ± 52.7 | 1.65 ± 1.40 |
| AFS no admission | 100.00 ± 0.00 | 0.00 ± 0.00 | 0.00 ± 0.00 | 100.00 ± 0.00 | 566.5 ± 20.8 | 2473.5 ± 55.4 | 1.41 ± 0.41 |
| Full AFS | 99.82 ± 0.41 | 0.18 ± 0.41 | 0.00 ± 0.00 | 100.00 ± 0.00 | 569.8 ± 19.0 | 2469.2 ± 47.5 | 2.04 ± 1.17 |

تفسیر: این workload در ناحیه سقف قرار دارد. تفاوت های کوچک در latency یا PSI نباید به ادعای برتری عمومی تبدیل شوند. نتیجه مهم این بخش این است که AFS در light load افت بنیادی ایجاد نمی کند.

### 11.2 Saturated workload

| Method | Goodput % | Accepted miss % | Rejection % | Completion % | P50 ms | P95 ms | PSI avg10 |
|---|---:|---:|---:|---:|---:|---:|---:|
| EEVDF | 38.87 ± 6.62 | 61.13 ± 6.62 | 0.00 ± 0.00 | 100.00 ± 0.00 | 4383.3 ± 481.9 | 20958.4 ± 783.1 | 88.20 ± 5.81 |
| RustLand | 83.57 ± 13.06 | 16.43 ± 13.06 | 0.00 ± 0.00 | 100.00 ± 0.00 | 2001.4 ± 700.4 | 25184.2 ± 1062.9 | 89.81 ± 5.78 |
| AFS no admission | 53.48 ± 8.14 | 46.52 ± 8.14 | 0.00 ± 0.00 | 100.00 ± 0.00 | 3871.3 ± 512.5 | 21805.7 ± 1069.8 | 87.55 ± 5.54 |
| Full AFS | 82.17 ± 3.08 | 7.01 ± 0.94 | 18.05 ± 2.34 | 81.95 ± 2.34 | 1857.6 ± 187.0 | 6186.9 ± 578.8 | 35.12 ± 2.95 |

AFS و RustLand از نظر headline goodput در یک محدوده قرار دارند و CI RustLand گسترده است. بنابراین ادعای برتری آماری قطعی AFS نسبت به RustLand از نظر goodput موجه نیست.

مزیت قابل دفاع AFS در saturated روی کیفیت accepted work و کنترل pressure دیده می شود:

- accepted miss: `7.01 ± 0.94%`
- P95: `6186.9 ± 578.8 ms`
- PSI avg10: `35.12 ± 2.95`

اما این به قیمت:

- rejection: `18.05 ± 2.34%`
- completion: `81.95 ± 2.34%`

به دست آمده است.

### 11.3 Overload workload

| Method | Goodput % | Accepted miss % | Rejection % | Completion % | P50 ms | P95 ms | PSI avg10 |
|---|---:|---:|---:|---:|---:|---:|---:|
| EEVDF | 0.00 ± 0.00 | 100.00 ± 0.00 | 0.00 ± 0.00 | 100.00 ± 0.00 | 70358.7 ± 561.0 | 130276.9 ± 638.8 | 98.68 ± 0.16 |
| RustLand | 0.00 ± 0.00 | 100.00 ± 0.00 | 0.00 ± 0.00 | 100.00 ± 0.00 | 64615.1 ± 441.0 | 143215.0 ± 1171.4 | 99.17 ± 0.09 |
| AFS no admission | 1.89 ± 0.31 | 98.11 ± 0.31 | 0.00 ± 0.00 | 100.00 ± 0.00 | 70085.1 ± 485.9 | 130496.8 ± 623.0 | 98.56 ± 0.11 |
| Full AFS | 13.32 ± 1.58 | 37.01 ± 6.36 | 79.38 ± 0.45 | 20.62 ± 0.45 | 1841.4 ± 153.1 | 7025.8 ± 662.3 | 87.66 ± 1.89 |

در overload، EEVDF و RustLand همه taskها را تکمیل می کنند، اما deadline goodput آنها صفر است و P95 به حدود 130 تا 143 ثانیه می رسد. AFS بدون admission نیز فقط `1.89%` goodput دارد.

AFS کامل رفتار متفاوتی دارد:

- offered deadline goodput: `13.32 ± 1.58%`
- accepted miss: `37.01 ± 6.36%`
- P95: `7025.8 ± 662.3 ms`
- rejection: `79.38 ± 0.45%`
- completion: `20.62 ± 0.45%`

این نتیجه نشان می دهد admission جلوی انباشته شدن نامحدود backlog را می گیرد و کیفیت پذیرفته شده را بهتر می کند. در عین حال rejection تقریبا 79 درصدی باید در مرکز تفسیر باقی بماند.

---

## 12. ارزیابی ثانویه

### 12.1 Ablation - حذف aging

| Workload | Goodput no-aging % | Goodput Full AFS % | Accepted miss no-aging % | Accepted miss Full AFS % | Rejection no-aging % | Rejection Full AFS % | P95 no-aging ms | P95 Full AFS ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Saturated | 83.04 | 82.17 | 7.72 | 7.01 | 16.69 | 18.05 | 6439.7 | 6186.9 |
| Overload | 10.56 | 13.32 | 51.24 | 37.01 | 78.97 | 79.38 | 6357.2 | 7025.8 |

حذف aging در saturated اثر بزرگی بر headline goodput ندارد، اما در overload accepted miss را از `37.01%` به `51.24%` افزایش می دهد و goodput را از `13.32%` به `10.56%` کاهش می دهد. این نتیجه با نقش aging به عنوان مکانیزم کاهش starvation و حفظ کیفیت در بار بسیار سنگین سازگار است.

### 12.2 Ablation - حذف application deadline

| Workload | Goodput no-app-deadline % | Goodput Full AFS % | Accepted miss no-app-deadline % | Accepted miss Full AFS % | Rejection no-app-deadline % | Rejection Full AFS % | P95 no-app-deadline ms | P95 Full AFS ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Saturated | 58.70 | 82.17 | 15.54 | 7.01 | 34.29 | 18.05 | 7151.0 | 6186.9 |
| Overload | 0.40 | 13.32 | 99.02 | 37.01 | 60.52 | 79.38 | 25495.5 | 7025.8 |

این ablation قوی ترین evidence ثانویه پروژه است. حذف application deadline باعث افت محسوس goodput و افزایش accepted miss می شود. نتیجه از این ادعا حمایت می کند که deadline/laxity metadata بخش واقعی policy است و تنها metadata تزئینی نیست.

### 12.3 Periodic/sporadic comparison

| Method | Deadline goodput % | Accepted miss % | Rejection % | Completion % | P95 ms |
|---|---:|---:|---:|---:|---:|
| EEVDF | 99.33 | 0.67 | 0.00 | 100.00 | 111.0 |
| Full AFS | 99.87 | 0.13 | 0.00 | 100.00 | 104.2 |
| SCHED_DEADLINE | 96.53 | 3.47 | 0.00 | 100.00 | 147.8 |

AFS در این workload frozen بهترین اعداد این جدول را دارد، اما این نتیجه نباید به ادعای برتری عمومی AFS بر Linux SCHED_DEADLINE تعمیم داده شود. workload، period، deadline و resource partition آزمایش خاص و frozen هستند.

### 12.4 نکته مهندسی SCHED_DEADLINE

در مسیر secondary evaluation یک محدودیت مهم kernel آشکار شد. SCHED_DEADLINE نیاز دارد affinity task کل root-domain مربوط را پوشش دهد. اجرای مستقیم با affinity `2-5` تا زمانی که root-domain متناظر ساخته نشده بود با `EPERM` شکست می خورد.

cpuset partition root برای CPUهای `2-5` ساخته شد. سپس یک باگ ظریف دیگر دیده شد: worker پس از `SIGSTOP` به cpuset جدید منتقل می شد؛ affinity به `2-5` تغییر می کرد ولی `task_cpu()` داخلی kernel هنوز CPU قبلی را نشان می داد. diagnostic نشان داد:

- stopped task بعد از move: `psr=0`, allowed `2-5`, SCHED_DEADLINE = EPERM
- پس از یک migration واقعی: `psr=2`, allowed `2-5`, SCHED_DEADLINE = OK

راه حل نهایی، ورود live pre-exec helper به root-domain صحیح قبل از exec کردن worker واقعی بود. worker سپس در همان root-domain register و stop می شد و policy با موفقیت اعمال می شد. این fix فقط در مسیر secondary SCHED_DEADLINE استفاده شد و primary frozen را تغییر نداد.

---

## 13. مطالعه موردی Fusion

Fusion pipeline با روش proposed اجرا شد.

نتیجه authoritative:

- total tasks: 120
- completed: 45
- rejected: 75
- failed: 0
- deadline misses: 19

این اجرا نشان می دهد مسیر کامل AFS، admission و pipeline ناهمگن می توانند end-to-end اجرا شوند. اما Fusion تنها یک case study است و `n=10` ندارد؛ بنابراین شمارش های آن نباید مانند primary matrix یا periodic benchmark به population estimate تبدیل شوند.

---

## 14. پاسخ به پرسش های پژوهش

### پاسخ RQ1

بله. در light load همه روش ها تقریبا 100 درصد goodput دارند و AFS افت بنیادی ایجاد نمی کند. این نتیجه یک no-harm/tie result است.

### پاسخ RQ2

تا حد قابل توجهی بله. در saturated، AFS goodput نزدیک RustLand دارد اما accepted miss، P95 و workload PSI پایین تری نشان می دهد. این کیفیت با حدود 18 درصد rejection به دست می آید.

### پاسخ RQ3

بله، اما با trade-off روشن. در overload، admission backlog و tail latency را مهار می کند و goodput را نسبت به AFS بدون admission افزایش می دهد، اما حدود 79 درصد بار را رد می کند.

### پاسخ RQ4

بله. حذف application deadline اثر شدید دارد. حذف aging نیز در overload accepted miss را بدتر می کند.

### پاسخ RQ5

در workload frozen periodic/sporadic، AFS نتیجه بهتری از EEVDF و SCHED_DEADLINE ثبت کرده است. این نتیجه scoped است و نباید عمومی سازی شود.

### پاسخ RQ6

بله. Fusion pipeline با 120 task اجرا شده و مسیر کامل proposed بدون failed task در سطح generator پایان یافته است، هرچند admission تعداد زیادی task را رد کرده است.

---

## 15. بحث

### 15.1 چرا goodput به تنهایی کافی نیست

اگر rejected task از denominator حذف شود، scheduler می تواند با رد کردن گسترده workload، goodput پذیرفته شده را مصنوعی بالا نشان دهد. قرارداد این پروژه rejected deadline task را در offered denominator نگه می دارد. به همین دلیل `deadline_goodput` همراه با `rejection_rate` و `completion_rate` معنی پیدا می کند.

### 15.2 admission به عنوان load shedding

AFS overload را با انجام همه کارها حل نمی کند. در عوض بخشی از workload را حذف می کند تا accepted set از نظر deadline و latency قابل کنترل تر بماند. این رفتار برای برخی سیستم ها مناسب و برای برخی نامناسب است. اگر application semantics اجازه drop/reject ندهد، این trade-off قابل استفاده نیست.

### 15.3 تفاوت saturated و overload

در saturated هنوز رقابت معنادار بین schedulerها وجود دارد و RustLand goodput بالایی ثبت می کند. در overload شدید، baselineها 100 درصد taskها را تکمیل می کنند ولی تقریبا همه deadlineها را از دست می دهند. این تفاوت نشان می دهد completion و deadline success دو مفهوم جدا هستند.

### 15.4 application metadata

Ablation بدون deadline نشان می دهد policy بدون metadata کاربردی به شدت ضعیف تر می شود. بنابراین اگر application نتواند deadline معنادار ارائه کند، بخشی از مزیت AFS نیز از بین می رود.

---

## 16. محدودیت ها و تهدیدهای اعتبار

### 16.1 محیط تک ماشین

همه نتایج روی یک Debian ARM64 VM با 6 CPU منطقی گرفته شده اند. نتایج به صورت خودکار به x86، NUMA، ماشین های چند socket یا bare-metal تعمیم داده نمی شوند.

### 16.2 workloadهای synthetic/frozen

workloadهای primary و periodic مصنوعی و برای آشکار کردن رفتار policy طراحی شده اند. نتیجه آنها evidence مهندسی است و جای ارزیابی روی workloadهای production واقعی را نمی گیرد.

### 16.3 تعداد repetition

برای گروه های آماری از 10 repetition استفاده شده است. این تعداد برای Student-t CI مناسب است، اما برای برآورد دقیق tail distributionهای بسیار سنگین محدودیت دارد.

### 16.4 Fusion تک اجرا

Fusion مطالعه موردی است و confidence interval ندارد.

### 16.5 overhead

پروژه overhead داخلی AFS را اندازه می گیرد، اما از آن برای ادعای مستقیم cross-method overhead استفاده نمی کند، زیرا instrumentation بین روش ها یکسان نیست.

### 16.6 SCHED_DEADLINE

مقایسه SCHED_DEADLINE به configuration frozen periodic/sporadic و cpuset root-domain مخصوص آزمایش محدود است.

### 16.7 admission semantics

rejection در برخی کاربردها قابل قبول و در برخی غیرقابل قبول است. بنابراین موفقیت AFS در overload به امکان application-level admission/load shedding وابسته است.

---

## 17. Reproducibility و provenance

### 17.1 Experimental freeze

- branch آزمایشی: `benchmark-engineering`
- freeze commit: `edcd8eb347f998fc68c705ca770eaa645c8e78be`
- tag: `experimental-freeze-v1`

### 17.2 Final documentation branch

- branch: `paper-finalization`
- secondary snapshot commit: `6747ba56fdde9e74961eeae3af045a6009d56eee`
- Fusion correction commit: `88931fbd281f8f69f540f016184610959290afd0`
- final Results commit: `b39c84d044bf4d942bcb0e64f0b6a6ae4b7a4972`
- final Proposal commit: `e5578ac29de211c6943414e62cd6b262f6e34bd5`

### 17.3 Tracked evidence

Primary:

- `docs/final-results/README.md`
- `docs/final-results/FINAL_RESULTS_SUMMARY.md`
- `docs/final-results/FINAL_INTEGRITY.md`
- `docs/final-results/aggregate-summary.csv`
- `docs/final-results/run-summary.csv`
- `docs/final-results/raw-artifact-sha256.txt`

Secondary:

- `docs/secondary-results/SECONDARY_RESULTS_SUMMARY.md`
- `docs/secondary-results/SECONDARY_INTEGRITY.md`
- aggregate و run-level CSVها
- Fusion summary و provenance
- 32 publication plot
- `docs/secondary-results/sha256.txt`

Publication-facing results:

- `docs/FINAL_RESULTS_SECTION.md`
- `report/FINAL_RESULTS_SECTION_FA.md`

Proposal:

- `docs/FINAL_PROPOSAL_REFRESH_FA.md`
- `docs/final-proposal-fa.pdf`

---

## 18. قواعد نهایی تفسیر

برای جلوگیری از overclaim، ارائه و مقاله باید این قواعد را رعایت کنند:

- goodput همیشه همراه rejection، completion و accepted miss گزارش شود؛
- light workload یک floor/tie regime است؛
- در saturated گفته شود AFS از نظر goodput تقریبا هم سطح RustLand است و مزیت اصلی روی accepted miss، P95 و pressure با هزینه rejection دیده می شود؛
- در overload، AFS به عنوان admission/load-shedding trade-off معرفی شود؛
- pilot و diagnostic برای claim نهایی استفاده نشوند؛
- از internal AFS counters برای cross-method overhead claim استفاده نشود؛
- periodic/SCHED_DEADLINE فقط به workload frozen خودش محدود شود؛
- Fusion فقط case-study evidence است.

---

## 19. نتیجه گیری

AFS نشان می دهد که ترکیب deadline-aware scheduling و admission control می تواند در شرایط saturation و overload رفتار سیستم را از «تکمیل دیرهنگام تقریبا همه taskها» به «پذیرش کنترل شده بخشی از بار با کیفیت deadline بهتر» تغییر دهد.

در light load این سیاست هزینه عملکردی بنیادی نشان نمی دهد. در saturated، headline goodput AFS با RustLand قابل مقایسه است، در حالی که accepted miss، P95 و workload pressure کاهش می یابند؛ هزینه این رفتار rejection است. در overload، admission اثر بسیار قوی تری دارد: goodput و tail latency بهتر می شوند اما بخش بزرگی از workload رد می شود.

Ablationها نشان می دهند application deadline/laxity یک جزء اصلی policy است و aging نیز در سخت ترین regime به کیفیت deadline کمک می کند. مقایسه periodic/sporadic نشان می دهد AFS در configuration frozen پروژه می تواند با EEVDF و SCHED_DEADLINE رقابت کند. Fusion نیز قابلیت اجرای end-to-end مسیر پیشنهادی را نشان می دهد.

بنابراین مهم ترین نتیجه پروژه نه «برتری مطلق scheduler» بلکه یک نتیجه دقیق تر است: **AFS یک مکانیزم قابل اندازه گیری برای تبدیل overload کنترل نشده به overload مدیریت شده ارائه می کند، مشروط بر اینکه application semantics اجازه admission و rejection صریح را بدهد.**

---

## 20. وضعیت نهایی پروژه

در زمان تهیه این گزارش:

- primary evaluation: complete
- secondary evaluation: complete
- Fusion case study: complete
- statistical aggregation: complete
- publication plots: complete
- final Results section: complete
- final Proposal refresh: complete
- final report: complete
- final PowerPoint results slides: pending
- professor demonstration recording: pending
- conference-paper draft: pending

مرحله بعدی پروژه، به روزرسانی PowerPoint نهایی با نتایج frozen و سپس آماده سازی demonstration recording است.
