# مجموعه ارزیابی

مجموعه ارزیابی پروژه از فایل های تنظیمات TOML، سناریوهای بار کاری و مطالعه موردی Fusion تشکیل شده است.

## فایل های تنظیمات

| فایل | هدف |
|---|---|
| `configs/smoke.toml` | بررسی صحت اولیه |
| `configs/light.toml` | بار سبک و کمتر از ظرفیت |
| `configs/saturated.toml` | بار نزدیک ظرفیت |
| `configs/overload.toml` | بار اضافه و انفجاری |
| `configs/periodic.toml` | مقایسه منصفانه با `SCHED_DEADLINE` |
| `configs/fusion-pipeline.toml` | مطالعه موردی Massive Data Fusion |

## کلاس های کاری

- `critical`: وظایف حساس با Deadline کوتاه
- `interactive`: وظایف پاسخ گویی و تعاملی
- `batch`: وظایف پس زمینه بدون Deadline سخت

## Baselineها

- EEVDF
- `SCHED_DEADLINE`
- `scx_rustland`
- روش پیشنهادی

## Ablationها

- بدون Aging
- بدون Admission Control
- بدون Application Deadline و با Virtual Deadline مشابه Rustland

## نتایج نمونه

پوشه `results/sample` شامل داده های ساختگی برای تست ابزار تحلیل است. نتایج واقعی باید در `results/real` ذخیره شوند.
