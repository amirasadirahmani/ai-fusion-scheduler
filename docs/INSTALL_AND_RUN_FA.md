# راهنمای نصب و اجرای پروژه

این سند نسخه اجرایی کوتاه برای Linux مرجع و macOS است. برای جزئیات بیشتر، فایل های زیر را ببین:

- `docs/MACBOOK_M1_PRO_GUIDE_FA.md`
- `docs/BUILD_ON_TARGET.md`
- `docs/SCX_INTEGRATION.md`
- `docs/BASELINE_PROTOCOLS.md`
- `docs/VALIDATION_STATUS.md`

## 1. اجرای پیشنهادی روی Linux مرجع

سیستم مرجع پروژه:

```text
Linux kernel: 6.12.95
sched_ext: فعال
Rust: 1.82.0
Python: 3.12.x
CUDA: استفاده نمی شود
PyTorch: استفاده نمی شود
```

### 1.1 بررسی محیط

```bash
./scripts/check_environment.sh
./scripts/collect_system_info.sh results/environment-before.txt
```

### 1.2 نصب وابستگی های Ubuntu/Debian

```bash
sudo ./scripts/install_deps_ubuntu.sh
```

### 1.3 ساخت کاربرانpace

```bash
./scripts/build_userspace.sh
```

### 1.4 ساخت بخش sched_ext

```bash
./scripts/build_sched_ext.sh
```

### 1.5 Smoke Test

```bash
sudo ./scripts/run_smoke_test.sh
```

اگر مشکل رخ داد:

```bash
sudo ./scripts/emergency_stop.sh
```

## 2. اجرای شبیه ساز بدون کرنل

```bash
cargo run -p afs-policy-simulator -- --config configs/smoke.toml --scheduler proposed --out-dir results/smoke/proposed
cargo run -p afs-policy-simulator -- --config configs/light.toml --scheduler proposed --out-dir results/light/proposed
cargo run -p afs-policy-simulator -- --config configs/overload.toml --scheduler proposed --out-dir results/overload/proposed
```

## 3. اجرای ماتریس سناریوها

```bash
./scripts/run_simulator_matrix.sh
./scripts/run_ablations.sh
./scripts/run_fusion_case.sh
```

## 4. تحلیل نتایج

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -r analysis/requirements-lock.txt
python analysis/analyze_results.py results/**/tasks.csv --out results/summary
python analysis/plot_results.py results/summary/aggregate-summary.csv --out-dir results/summary/plots
```

## 5. خروجی های مهم

```text
results/**/tasks.csv
results/**/summary.json
results/**/scheduler-stats.json
results/summary/aggregate-summary.csv
results/summary/plots/*.png
```

## 6. نکته مهم درباره نتایج نمونه

پوشه `results/sample` فقط برای آزمایش ابزار تحلیل و قالب گزارش است. این نتایج ساختگی هستند و نباید به عنوان نتیجه نهایی پژوهش گزارش شوند.
