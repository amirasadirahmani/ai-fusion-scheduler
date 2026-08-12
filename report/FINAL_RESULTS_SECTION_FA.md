# بخش نهایی نتایج — AI Fusion Scheduler

## دامنه ارزیابی و قرارداد آماری

ماتریس اصلی نهایی شامل **۱۲۰ اجرای معتبر** است: سه سطح بار، چهار روش و ده
تکرار برای هر ترکیب. واحد آماری «تکرار» است؛ هر معیار ابتدا در سطح هر run
محاسبه و سپس میان ده تکرار با میانگین و نیم‌عرض فاصله اطمینان ۹۵ درصد
Student-t (`n=10`، `df=9`، `t=2.262`) تجمیع شده است. taskهای deadlineدار که
توسط admission رد می‌شوند همچنان در مخرج offered deadline goodput باقی
می‌مانند؛ بنابراین goodput همیشه همراه با accepted miss، rejection و
completion گزارش می‌شود.

ارزیابی اصلی چهار روش EEVDF، RustLand اصلی، AFS بدون admission و AFS کامل را
مقایسه می‌کند. ارزیابی ثانویه نیز شامل ۲۰ اجرای no-aging، ۲۰ اجرای
no-application-deadline، ۳۰ اجرای اندازه‌گیری‌شده periodic/sporadic به‌علاوه
۱۰ اجرای manifest-seed و یک مطالعه موردی Fusion کامل است. هر ۸۰ validation
استاندارد ثانویه با وضعیت OK پایان یافته‌اند. داده‌های pilot و diagnostic در
ادعاهای نهایی استفاده نمی‌شوند.

## نتایج ماتریس اصلی

| Workload | Method | Goodput % | Accepted miss % | Rejection % | Completion % | P95 ms | Workload PSI avg10 |
|---|---|---:|---:|---:|---:|---:|---:|
| Light | AFS no admission | 100.00 ± 0.00 | 0.00 ± 0.00 | 0.00 ± 0.00 | 100.00 ± 0.00 | 2473.5 ± 55.4 | 0.80 ± 0.56 |
| Light | EEVDF | 100.00 ± 0.00 | 0.00 ± 0.00 | 0.00 ± 0.00 | 100.00 ± 0.00 | 2447.0 ± 46.6 | 0.18 ± 0.21 |
| Light | Full AFS | 99.82 ± 0.41 | 0.18 ± 0.41 | 0.00 ± 0.00 | 100.00 ± 0.00 | 2469.2 ± 47.5 | 2.31 ± 3.53 |
| Light | RustLand | 100.00 ± 0.00 | 0.00 ± 0.00 | 0.00 ± 0.00 | 100.00 ± 0.00 | 2470.2 ± 52.7 | 0.84 ± 0.92 |
| Saturated | AFS no admission | 53.48 ± 8.14 | 46.52 ± 8.14 | 0.00 ± 0.00 | 100.00 ± 0.00 | 21805.7 ± 1069.8 | 83.09 ± 14.26 |
| Saturated | EEVDF | 38.87 ± 6.62 | 61.13 ± 6.62 | 0.00 ± 0.00 | 100.00 ± 0.00 | 20958.4 ± 783.1 | 82.03 ± 14.27 |
| Saturated | Full AFS | 82.17 ± 3.08 | 7.01 ± 0.94 | 18.05 ± 2.34 | 81.95 ± 2.34 | 6186.9 ± 578.8 | 20.43 ± 4.99 |
| Saturated | RustLand | 83.57 ± 13.06 | 16.43 ± 13.06 | 0.00 ± 0.00 | 100.00 ± 0.00 | 25184.2 ± 1062.9 | 84.84 ± 13.94 |
| Overload | AFS no admission | 1.89 ± 0.31 | 98.11 ± 0.31 | 0.00 ± 0.00 | 100.00 ± 0.00 | 130496.8 ± 623.0 | 94.70 ± 3.69 |
| Overload | EEVDF | 0.00 ± 0.00 | 100.00 ± 0.00 | 0.00 ± 0.00 | 100.00 ± 0.00 | 130276.9 ± 638.8 | 96.11 ± 2.78 |
| Overload | Full AFS | 13.32 ± 1.58 | 37.01 ± 6.36 | 79.38 ± 0.45 | 20.62 ± 0.45 | 7025.8 ± 662.3 | 82.53 ± 4.33 |
| Overload | RustLand | 0.00 ± 0.00 | 100.00 ± 0.00 | 0.00 ± 0.00 | 100.00 ± 0.00 | 143215.0 ± 1171.4 | 99.80 ± 0.16 |

### بار سبک

در بار سبک هر چهار روش عملاً در ناحیه سقف نزدیک به ۱۰۰٪ قرار دارند. AFS کامل
goodput برابر 99.82 ± 0.41٪،
accepted miss برابر 0.18 ± 0.41٪،
rejection برابر 0.00 ± 0.00٪ و
completion برابر 100.00 ± 0.00٪ دارد.
این سناریو باید به‌عنوان «عدم افت محسوس/برابری در بار سبک» تفسیر شود، نه
برتری scheduler.

### بار اشباع

در saturated، AFS کامل goodput برابر
82.17 ± 3.08٪ و RustLand برابر
83.57 ± 13.06٪ دارد. فاصله اطمینان
RustLand گسترده و با نتیجه AFS هم‌پوشان است؛ بنابراین ادعای برتری قطعی AFS
از نظر goodput نسبت به RustLand موجه نیست. نکته مهم‌تر این است که AFS کامل
accepted miss را به 7.01 ± 0.94٪،
P95 را به 6186.9 ± 578.8 ms و workload
PSI avg10 را به 20.43 ± 4.99
می‌رساند، اما در مقابل
18.05 ± 2.34٪ از بار ورودی را رد
می‌کند.

### اضافه‌بار

در overload اثر admission به‌وضوح دیده می‌شود. AFS کامل goodput برابر
13.32 ± 1.58٪ دارد، در حالی که AFS
بدون admission به 1.89 ± 0.31٪
می‌رسد. P95 در AFS کامل
7025.8 ± 662.3 ms است؛ با این حال
rejection برابر 79.38 ± 0.45٪ و
completion برابر 20.62 ± 0.45٪ است.
بنابراین این نتیجه باید «مهار کنترل‌شده اضافه‌بار با load shedding» توصیف
شود و نه یک بهبود بدون هزینه.

![Goodput ماتریس اصلی](../docs/secondary-results/plots/primary/goodput.png)

![P95 ماتریس اصلی](../docs/secondary-results/plots/primary/p95-response.png)

## Ablation: حذف aging

| Workload | Variant goodput % | Full AFS goodput % | Variant accepted miss % | Full AFS accepted miss % | Variant rejection % | Full AFS rejection % | Variant P95 ms | Full AFS P95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Saturated | 83.04 ± 2.94 | 82.17 ± 3.08 | 7.72 ± 2.18 | 7.01 ± 0.94 | 16.69 ± 2.28 | 18.05 ± 2.34 | 6439.7 ± 736.3 | 6186.9 ± 578.8 |
| Overload | 10.56 ± 2.09 | 13.32 ± 1.58 | 51.24 ± 8.20 | 37.01 ± 6.36 | 78.97 ± 0.50 | 79.38 ± 0.45 | 6357.2 ± 278.6 | 7025.8 ± 662.3 |

حذف aging در saturated اثر بزرگی بر goodput اصلی ندارد، اما در overload
goodput به 10.56 ± 2.09٪ کاهش و
accepted miss به 51.24 ± 8.20٪
افزایش می‌یابد. بنابراین شواهد، نقش aging را بیشتر در جلوگیری از
starvation/افت کیفیت deadline در بار بسیار سنگین نشان می‌دهند.

## Ablation: حذف application-deadline awareness

| Workload | Variant goodput % | Full AFS goodput % | Variant accepted miss % | Full AFS accepted miss % | Variant rejection % | Full AFS rejection % | Variant P95 ms | Full AFS P95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Saturated | 58.70 ± 5.38 | 82.17 ± 3.08 | 15.54 ± 4.81 | 7.01 ± 0.94 | 34.29 ± 2.76 | 18.05 ± 2.34 | 7151.0 ± 656.2 | 6186.9 ± 578.8 |
| Overload | 0.40 ± 0.19 | 13.32 ± 1.58 | 99.02 ± 0.46 | 37.01 ± 6.36 | 60.52 ± 0.00 | 79.38 ± 0.45 | 25495.5 ± 371.0 | 7025.8 ± 662.3 |

این ablation قوی‌ترین اثر ثانویه را نشان می‌دهد. بدون application deadline،
goodput در saturated به 58.70 ± 5.38٪
و در overload به 0.40 ± 0.19٪
می‌رسد. accepted miss در overload نیز
99.02 ± 0.46٪ است. بنابراین
deadline/laxity اطلاعاتی بخش مؤثر سیاست AFS است و صرفاً metadata تزئینی نیست.

![Goodput بدون application deadline](../docs/secondary-results/plots/no-application-deadline/goodput.png)

## مقایسه periodic/sporadic

| Method | Goodput % | Accepted miss % | Rejection % | Completion % | P95 ms |
|---|---:|---:|---:|---:|---:|
| EEVDF | 99.33 ± 0.67 | 0.67 ± 0.67 | 0.00 ± 0.00 | 100.00 ± 0.00 | 111.0 ± 11.0 |
| Full AFS | 99.87 ± 0.30 | 0.13 ± 0.30 | 0.00 ± 0.00 | 100.00 ± 0.00 | 104.2 ± 6.9 |
| SCHED_DEADLINE | 96.53 ± 1.29 | 3.47 ± 1.29 | 0.00 ± 0.00 | 100.00 ± 0.00 | 147.8 ± 16.3 |

در workload اختصاصی periodic/sporadic، AFS کامل goodput
99.87 ± 0.30٪ و accepted miss
0.13 ± 0.30٪ دارد؛ EEVDF به
ترتیب 99.33 ± 0.67٪ و
0.67 ± 0.67٪، و SCHED_DEADLINE به
ترتیب 96.53 ± 1.29٪ و
3.47 ± 1.29٪ ثبت کرده‌اند. این نتیجه
فقط برای workload frozen همین آزمایش معتبر است و نباید به برتری عمومی نسبت
به همه workloadهای real-time لینوکس تعمیم داده شود.

![Goodput periodic/sporadic](../docs/secondary-results/plots/periodic/goodput.png)

## مطالعه موردی Fusion

سناریوی کامل Fusion شامل **120 task** بود:
**45 completed**، **75 rejected**، **0 failed** و
**19 deadline miss**.

این شمارش‌ها مستقیماً از `summary.json` نهایی generator گرفته شده‌اند و با
120 ردیف موجود در `tasks.csv` نیز cross-check شده‌اند. این اجرا یک
مطالعه موردی تک‌سناریویی است و نه یک گروه آماری ده‌تکراری؛ بنابراین برای
نشان‌دادن اجرای end-to-end خط لوله ناهمگن با admission فعال در AFS استفاده
می‌شود و شمارش‌های خام آن به‌عنوان برآورد آماری قابل تعمیم گزارش نمی‌شوند.

## قواعد تفسیر نهایی

- offered deadline goodput همیشه همراه rejection، completion و accepted miss
  گزارش شود.
- بار light به‌عنوان tie/floor تفسیر شود.
- در saturated گفته شود AFS از نظر goodput تقریباً هم‌سطح RustLand است، ولی
  accepted miss، tail latency و pressure را با هزینه rejection کاهش می‌دهد؛
  ادعای برتری آماری قطعی goodput مطرح نشود.
- در overload، rejection بالا جزء اصلی trade-off admission است و نباید حذف
  یا کم‌رنگ شود.
- pilot و diagnostic برای ادعاهای نهایی استفاده نشوند.
- از شمارنده‌های داخلی AFS برای ادعای مستقیم overhead بین روش‌ها استفاده
  نشود.
- نتیجه periodic/SCHED_DEADLINE فقط به workload frozen خودش و Fusion فقط به
  همان case study محدود بماند.

## منشأ داده‌ها

- نتایج اصلی: `docs/final-results/`
- نتایج ثانویه و نمودارها: `docs/secondary-results/`
- freeze آزمایش: `experimental-freeze-v1`
- commit freeze: `edcd8eb347f998fc68c705ca770eaa645c8e78be`
