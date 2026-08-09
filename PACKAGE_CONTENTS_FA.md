# فهرست تحویلی پروژه

| مورد درخواستی | مسیر در بسته | وضعیت |
|---|---|---|
| فایل تنظیمات نمونه | `configs/*.toml` | موجود |
| مجموعه ارزیابی | `docs/EVALUATION_DATASET_FA.md` و `configs/` | موجود |
| نتایج آزمایش ها | `results/sample` برای نمونه ساختگی؛ `results/real` برای نتایج واقعی | قالب و نمونه موجود؛ نتیجه واقعی نیازمند اجرا روی سیستم هدف است |
| گزارش نهایی | `report/FINAL_REPORT_FA.md` | قالب کامل قابل تکمیل موجود |
| راهنمای نصب و اجرا | `docs/INSTALL_AND_RUN_FA.md` و `docs/QUICK_START_FA.md` | موجود |
| راهنمای MacBook M1 Pro | `docs/MACBOOK_M1_PRO_GUIDE_FA.md` | موجود |
| اسلاید ارائه | `presentation/ai_fusion_scheduler_presentation.pptx` | موجود |
| Dependency Lock | `docs/DEPENDENCY_LOCK.md`, `analysis/requirements-lock.txt`, `rust-toolchain.toml` | موجود؛ `Cargo.lock` واقعی باید روی سیستم هدف با `cargo generate-lockfile` تولید شود |
| CUDA/PyTorch | در `docs/DEPENDENCY_LOCK.md` ذکر شده که استفاده نمی شوند | موجود |
