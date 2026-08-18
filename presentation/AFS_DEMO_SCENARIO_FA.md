# سناریوی دقیق 10 تا 15 دقیقه‌ای دمو با شماره اسلایدها

این فایل برای قرار گرفتن کنار PowerPoint نهایی پروژه است و باید در repository هم track شود.
Deck مربوطه:

`presentation/ai_fusion_scheduler_final_results_deck.pptx`

هدف این سناریو این است که در ضبط 10 تا 15 دقیقه‌ای، بدون اجرای دوباره benchmarkهای سنگین، داستان پروژه را از مسئله تا evidence نهایی توضیح بدهی و فقط artifactهای frozen داخل repository را نشان بدهی.

---

## برنامه کلی

| بازه زمانی | اسلاید | هدف |
|---:|---:|---|
| 0:00-0:45 | 1 | معرفی پروژه و claim اصلی |
| 0:45-1:45 | 2 | مسئله پژوهش و چرایی AFS |
| 1:45-2:45 | 3 | معماری و مسیر تصمیم گیری |
| 2:45-3:45 | 4 | قرارداد frozen evaluation |
| 3:45-4:45 | 5 | تعریف درست goodput همراه rejection |
| 4:45-5:35 | 6 | نتیجه Light |
| 5:35-6:45 | 7 | نتیجه Saturated |
| 6:45-8:00 | 8 | نتیجه Overload |
| 8:00-9:10 | 9 | Ablationها |
| 9:10-10:10 | 10 | Periodic/SCHED_DEADLINE |
| 10:10-11:00 | 11 | Fusion case study |
| 11:00-11:50 | 12 | Guardrailهای claim |
| 11:50-12:50 | 13 | Artifactها و traceability |
| 12:50-14:00 | 14 + Terminal | جمع بندی و commandهای نمایشی |
| 14:00-15:00 | Q/A | جواب سؤال احتمالی استاد |

---

## متن سخنرانی اسلاید به اسلاید

### اسلاید 1 - AI Fusion Scheduler

سلام. پروژه من **AI Fusion Scheduler یا AFS** است. هدف این پروژه طراحی و ارزیابی یک scheduler آزمایشی روی Linux است که برای workloadهای ناهمگن، اطلاعاتی مثل deadline، priority و laxity را وارد تصمیم scheduling می‌کند.

claim اصلی پروژه این نیست که AFS در همه شرایط بهترین scheduler است. claim دقیق‌تر این است که در فشار بالا، AFS می‌تواند overload کنترل نشده را به یک trade-off صریح load shedding تبدیل کند: کیفیت کار پذیرفته شده بهتر می‌شود، اما rejection هم کاملاً گزارش می‌شود.

---

### اسلاید 2 - Problem & thesis

مشکل اصلی این است که completion به تنهایی کافی نیست. ممکن است یک scheduler همه taskها را کامل کند، اما deadlineها مدت‌ها قبل از completion از دست رفته باشند.

در این پروژه به جای اینکه فقط throughput یا completion را ببینم، سه چیز را کنار هم گزارش می‌کنم:

- deadline goodput؛
- accepted miss؛
- rejection و completion.

AFS از application metadata، laxity/deadline scoring، aging و admission control استفاده می‌کند. ولی همه claimها regime-specific هستند و overclaim نمی‌کنم.

---

### اسلاید 3 - AFS architecture in one path

معماری AFS از چند مرحله تشکیل شده است.

ابتدا application metadata مثل class، priority، deadline و runtime estimate وارد سیستم می‌شود. سپس policy scoring براساس laxity، priority و aging انجام می‌شود. اگر pressure بالا باشد، admission می‌تواند task را delay یا reject کند.

نکته مهم این است که measurement هم اصلاح شده است. در نسخه frozen، PSI از cgroup workload خوانده می‌شود، نه از `/proc/pressure/cpu` سراسری؛ چون control plane می‌تواند PSI سراسری را آلوده کند.

---

### اسلاید 4 - Frozen evaluation contract

برای اینکه نتایج بعد از دیدن داده‌ها تغییر نکنند، evaluation را freeze کردم.

Primary evaluation شامل 120 اجرای validated است: سه workload، چهار روش و ده repetition.

روش‌ها:

- EEVDF؛
- RustLand؛
- AFS بدون admission؛
- Full AFS.

Secondary evaluation هم شامل 80 validation استاندارد است: ablationها، periodic/SCHED_DEADLINE و Fusion case study.

واحد آماری repetition است و نتایج با mean و Student-t 95% confidence interval گزارش شده‌اند.

---

### اسلاید 5 - Primary headline: goodput only makes sense with rejection

این اسلاید مهم‌ترین guardrail متریک را نشان می‌دهد.

اگر rejected taskها را از denominator حذف کنیم، scheduler می‌تواند با رد کردن بار زیاد ظاهراً خیلی خوب به نظر برسد. برای همین در تحلیل من، taskهای deadlineدار rejected همچنان در offered deadline denominator باقی می‌مانند.

بنابراین هر وقت goodput را می‌گویم، باید هم‌زمان rejection، completion و accepted miss را هم ببینیم.

---

### اسلاید 6 - Light load: no-harm floor regime

در light load همه روش‌ها نزدیک سقف 100 درصد هستند.

Full AFS حدود `99.82%` goodput دارد و rejection صفر است. این regime برای ادعای برتری نیست. تفسیر درست آن این است که AFS در بار سبک رفتار baselineها را به شکل بنیادی خراب نمی‌کند.

پس این اسلاید را به عنوان tie یا no-harm result می‌بینم.

---

### اسلاید 7 - Saturation: quality improves, but rejection is visible

در saturated، Full AFS حدود `82.17 ± 3.08%` goodput دارد و RustLand حدود `83.57 ± 13.06%`.

چون confidence interval RustLand گسترده و با AFS هم‌پوشان است، ادعای superiority از نظر goodput نمی‌کنم.

اما AFS accepted miss را به حدود `7.01%`، P95 را به حدود `6.19s` و workload PSI را به حدود `35.12` کاهش می‌دهد. هزینه این رفتار rejection حدود `18.05%` است.

پس نتیجه saturated یک quality-versus-rejection trade-off است.

---

### اسلاید 8 - Overload: controlled load shedding

در overload، baselineها همه taskها را کامل می‌کنند، اما deadline goodput آنها صفر است و P95 به حدود 130 تا 143 ثانیه می‌رسد.

Full AFS goodput را به `13.32%` می‌رساند و P95 را حدود `7.03s` نگه می‌دارد. اما rejection حدود `79.38%` است.

این دقیقاً نقطه اصلی پروژه است: AFS overload را با انجام همه کارها حل نمی‌کند؛ بلکه بخشی از بار را صریحاً reject می‌کند تا کیفیت کار پذیرفته شده قابل کنترل بماند.

---

### اسلاید 9 - Ablations show what the policy needs

Ablationها نشان می‌دهند کدام بخش policy واقعاً اثر دارد.

در no-aging، overload goodput از `13.32%` به `10.56%` کاهش می‌یابد و accepted miss از `37.01%` به `51.24%` می‌رسد.

در no-application-deadline اثر شدیدتر است. Saturated goodput به `58.70%` و overload goodput به `0.40%` سقوط می‌کند. این یعنی deadline/laxity فقط metadata تزئینی نیست و بخش مؤثر policy است.

---

### اسلاید 10 - Periodic/sporadic: scoped SCHED_DEADLINE comparison

در workload periodic/sporadic frozen، AFS با EEVDF و SCHED_DEADLINE مقایسه شد.

نتیجه:

- AFS: `99.87%` goodput؛
- EEVDF: `99.33%`؛
- SCHED_DEADLINE: `96.53%`.

این نتیجه را فقط برای همین workload frozen بیان می‌کنم و به همه workloadهای real-time لینوکس تعمیم نمی‌دهم.

در مسیر اجرا یک نکته kernel-level هم حل شد: SCHED_DEADLINE به root-domain صحیح نیاز دارد و stopped task باید قبل از policy application واقعاً وارد root-domain درست شده باشد.

---

### اسلاید 11 - Fusion case study: end-to-end AFS path

Fusion یک case study است، نه benchmark آماری ده‌تکراری.

نتیجه authoritative از `summary.json`:

- 120 task؛
- 45 completed؛
- 75 rejected؛
- 0 failed؛
- 19 deadline miss.

این نشان می‌دهد مسیر کامل AFS و admission می‌تواند یک pipeline ناهمگن را end-to-end اجرا کند. اما چون فقط یک case study است، از آن confidence interval یا claim آماری عمومی استخراج نمی‌کنم.

---

### اسلاید 12 - What we can safely claim

اینجا مرز claimها را دقیق می‌گویم.

می‌توانم بگویم AFS در light load افت بنیادی ایجاد نکرده است. در saturated، کیفیت accepted work و pressure را با هزینه rejection بهتر کرده است. در overload، admission کنترل‌شده ایجاد کرده و tail latency را از ده‌ها ثانیه به حدود چند ثانیه کاهش داده، اما با rejection بالا.

نمی‌توانم بگویم AFS همیشه از RustLand بهتر است یا همیشه از SCHED_DEADLINE بهتر است. اینها claimهای بیش از داده هستند.

---

### اسلاید 13 - Final artifacts are frozen and traceable

همه artifactهای نهایی داخل repository track شده‌اند:

- final primary results؛
- secondary results و 32 plot؛
- Results section؛
- Proposal؛
- Final Report؛
- PowerPoint؛
- سناریوی دمو.

این مهم است چون برای استاد می‌توانم zip کامل پروژه را بفرستم و همه evidenceها در مسیرهای ثابت داخل repository قرار دارند.

---

### اسلاید 14 - Next step: record the professor demo

در پایان می‌گویم benchmarkهای final را وسط فیلم دوباره اجرا نمی‌کنم.

دلیلش این است که final evaluation چندتکراری، frozen و دارای quiescence/provenance است. اجرای یک command بداهه وسط فیلم از نظر علمی جایگزین آن نمی‌شود.

در عوض repository clean، summaries، checksums و artifactهای نهایی را نشان می‌دهم.

---

## بخش Terminal داخل فیلم

### 1. وضعیت repo

```bash
cd ~/project/ai-fusion-scheduler-v0.2.0/ai-fusion-scheduler
git status --short
git log -6 --oneline --decorate
uname -r
cat /sys/kernel/sched_ext/state
```

اگر `git status --short` خروجی نداشت، بگو: «خروجی خالی یعنی tree clean است.»

### 2. highlights نتایج اصلی

```bash
grep -n -E 'Light|Saturated|Overload|82\.17|13\.32|79\.38'   docs/final-results/FINAL_RESULTS_SUMMARY.md
```

### 3. تعداد plotها

```bash
find docs/secondary-results/plots -type f -name '*.png' | wc -l
```

خروجی مورد انتظار:

```text
32
```

### 4. integrity ثانویه

```bash
(
  cd docs/secondary-results
  sha256sum -c sha256.txt | tail -n 5
)
```

```bash
cat docs/secondary-results/SECONDARY_INTEGRITY.md
```

### 5. Fusion

```bash
cat docs/secondary-results/FUSION_SUMMARY.md
```

### 6. فایل‌های نهایی برای ارسال

```bash
ls -lh   docs/final-proposal-fa.pdf   report/FINAL_REPORT_FA.md   presentation/ai_fusion_scheduler_final_results_deck.pptx   presentation/AFS_DEMO_SCENARIO_FA.md
```

---

## جواب سؤال‌های احتمالی استاد

### چرا rejection در overload اینقدر بالاست؟

چون AFS از admission برای load shedding استفاده می‌کند. اگر rejection را مخفی کنیم، نتیجه ناقص و گمراه‌کننده می‌شود. به همین دلیل goodput همیشه کنار rejection و completion گزارش شده است.

### آیا RustLand در saturated بهتر نیست؟

از نظر mean goodput کمی بالاتر است، ولی CI گسترده و هم‌پوشان دارد. claim من superiority goodput نیست. claim من کاهش accepted miss، P95 و pressure با هزینه rejection است.

### چرا baselineها completion بالا دارند ولی goodput پایین؟

completion یعنی task بالاخره تمام شده؛ goodput یعنی task deadlineدار به موقع تمام شده. در overload baselineها taskها را دیر تمام می‌کنند.

### چرا SCHED_DEADLINE فقط روی workload periodic/sporadic آمده؟

چون semantics آن با workload periodic/sporadic سازگارتر است. نتیجه scoped است و تعمیم عمومی نمی‌دهم.

### چرا Fusion فقط یک بار است؟

Fusion یک case study برای end-to-end path است، نه benchmark آماری. ادعاهای آماری از primary و secondary measured matrix می‌آیند.

### AI در نام پروژه یعنی مدل ML آموزش‌دیده داری؟

در نسخه فعلی claim من ML training نیست. هسته نهایی یک policy مهندسی deadline/laxity/priority/aging و admission است. claimها فقط بر اساس چیزی است که پیاده‌سازی و اندازه‌گیری شده است.

---

## چک‌لیست قبل از ضبط

- PowerPoint را از اسلاید 1 باز کن.
- Terminal font را حداقل 16 یا 18 بگذار.
- notificationها را خاموش کن.
- VM را چند دقیقه idle بگذار.
- `git status --short` باید خالی باشد.
- `cat /sys/kernel/sched_ext/state` باید `disabled` باشد.
- benchmark final را وسط فیلم اجرا نکن.
- Overload را همیشه با `79.38% rejection` توضیح بده.
- Fusion را case study بنام، نه benchmark آماری.
