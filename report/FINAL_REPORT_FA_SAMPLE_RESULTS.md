# گزارش کوتاه نتایج نمونه

این فایل فقط برای نشان دادن قالب گزارش نتایج است. داده های پوشه `results/sample` ساختگی هستند و نباید در گزارش نهایی پژوهش به عنوان نتیجه واقعی استفاده شوند.

## خلاصه نمونه

در نمونه ساختگی، روش `proposed` در سناریوی overload نرخ Deadline Miss کمتری نسبت به `eevdf` و `scx_rustland` نشان می دهد؛ اما این صرفا برای تست نمودارها و Pipeline تحلیل ساخته شده است.

برای تولید جدول واقعی:

```bash
python analysis/analyze_results.py results/real/**/tasks.csv --out results/summary
python analysis/make_markdown_table.py results/summary/aggregate-summary.csv > report/results-table.md
```
