# راهنمای سریع اجرای پروژه

این مخزن بخش‌های فضای کاربر، شبیه‌ساز سیاست، تولید بار، مطالعه موردی همجوشی داده، اتصال `scx_rustland_core`، اسکریپت‌های آزمایش و تحلیل نتایج را شامل می‌شود.

## ۱. نکته مهم

بخش‌های معمولی Rust را می‌توان روی هر Linux جدید کامپایل کرد، اما زمان‌بند واقعی فقط باید روی میزبان آزمایشِ تثبیت‌شده با Linux 6.12.95، دسترسی root، BTF و `sched_ext` اجرا شود. ابتدا از VM یا ماشین آزمایش جداگانه استفاده کنید و یک نشست SSH دوم باز نگه دارید.

## ۲. بررسی محیط

```bash
./scripts/check_environment.sh
./scripts/collect_system_info.sh results/environment-before.txt
```

پس از انتخاب نسخه سازگار `scx`، Commit یا Tag آن را در `ENVIRONMENT.md` ثبت کنید و در طول آزمایش‌ها تغییر ندهید.

## ۳. ساخت بخش‌های فضای کاربر

```bash
./scripts/build_userspace.sh
```

ابتدا شبیه‌ساز را اجرا کنید:

```bash
cargo run -p afs-policy-simulator -- \
  --config configs/saturated.toml \
  --output results/sim-saturated.csv
```

## ۴. ساخت زمان‌بند

```bash
./scripts/build_sched_ext.sh
```

اگر API نسخه تثبیت‌شده `scx` متفاوت بود، فقط لایه `crates/rustland-scheduler` را با نمونه `scx_rlfifo` همان نسخه تطبیق دهید و منطق سیاست را تغییر ندهید.

## ۵. Smoke Test ایمن

```bash
sudo ./scripts/run_smoke_test.sh
```

توقف اضطراری:

```bash
sudo ./scripts/emergency_stop.sh
```

## ۶. اجرای روش‌ها

```bash
./scripts/run_experiment.sh eevdf configs/light.toml results/eevdf-light
sudo ./scripts/run_experiment.sh rustland configs/light.toml results/rustland-light
sudo ./scripts/run_experiment.sh proposed configs/light.toml results/proposed-light
sudo ./scripts/run_experiment.sh sched-deadline configs/periodic.toml results/deadline-periodic
```

کنترل پذیرش پروژه برای Baselineها به‌صورت پیش‌فرض غیرفعال است تا مقایسه منصفانه بماند. رد شدن پهنای باند توسط خود `SCHED_DEADLINE` به‌عنوان نتیجه آزمایش ثبت می‌شود.

## ۷. مطالعه موردی Massive Data Fusion

```bash
sudo ./scripts/run_fusion_case.sh proposed \
  configs/fusion-pipeline.toml results/fusion-proposed
```

## ۸. تحلیل

```bash
python3 analysis/analyze_results.py results \
  --out results/run-summary.csv \
  --aggregate-out results/aggregate-summary.csv
python3 analysis/plot_results.py results/aggregate-summary.csv \
  --out-dir results/plots
```

## ۹. تعریف پایان موفق پروژه

- اجرای پایدار `scx_simple` و `scx_rustland` روی میزبان؛
- Build و Load شدن زمان‌بند پیشنهادی در حالت Partial؛
- اجرای EEVDF، `SCHED_DEADLINE`، `scx_rustland` و روش پیشنهادی؛
- اجرای سه Ablation اصلی؛
- تولید CSV خام، خلاصه آماری، نمودارها و مطالعه موردی Fusion؛
- ثبت نسخه کرنل، SHA مخزن `scx`، Toolchain و تنظیمات سیستم.
