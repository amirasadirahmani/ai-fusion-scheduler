# راهنمای اجرا و آموزش کدها روی MacBook M1 Pro

این راهنما برای اجرای بخش های قابل اجرای پروژه روی macOS و Apple Silicon نوشته شده است. توجه کن که بخش واقعی `sched_ext`، بارگذاری BPF، `SCHED_DEADLINE`، PSI لینوکس و `scx_rustland_core` روی macOS اجرا نمی شوند. مک بوک برای توسعه، شبیه سازی، Unit Test، تحلیل نتایج و آماده سازی گزارش مناسب است؛ اجرای نهایی Scheduler باید روی Linux 6.12.95 انجام شود.

## 1. چه چیزهایی روی Mac اجرا می شوند؟

روی MacBook M1 Pro می توانی این بخش ها را اجرا کنی:

- `policy-core`: محاسبه Laxity، Aging، امتیازدهی و Runtime Estimator
- `policy-simulator`: شبیه سازی سیاست بدون کرنل
- `workload-generator` در حالت dry-run یا synthetic mode
- `fusion-pipeline` در حالت آزمایشی CPU/Mem/I/O سبک
- تحلیل CSV و تولید نمودار با Python
- ویرایش و تست فایل های TOML
- آماده سازی گزارش نهایی و اسلایدها

روی macOS اجرا نمی شوند:

- `sched_ext`
- بارگذاری BPF Scheduler
- اجرای `scx_simple` یا `scx_rustland` واقعی
- Baseline واقعی `SCHED_DEADLINE`
- خواندن `/proc/pressure/cpu`
- اندازه گیری Dispatch واقعی هسته

## 2. نصب ابزارهای پایه روی macOS

ابتدا Command Line Tools را نصب کن:

```bash
xcode-select --install
```

اگر Homebrew نداری نصب کن، سپس:

```bash
brew update
brew install git rustup python@3.12 pkg-config cmake
```

Rust را فعال کن:

```bash
rustup-init -y
source "$HOME/.cargo/env"
rustup toolchain install 1.82.0
rustup default 1.82.0
rustc --version
cargo --version
```

نسخه مرجع پروژه Rust 1.82.0 است. اگر روی مک نسخه جدیدتر نصب شد، برای بازتولید نتایج از فایل `rust-toolchain.toml` پروژه استفاده کن.

## 3. ساخت محیط Python برای تحلیل نتایج

پیشنهاد ساده با `venv`:

```bash
cd ai-fusion-scheduler
python3.12 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install -r analysis/requirements-lock.txt
```

اگر `python3.12` نداری:

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install -r analysis/requirements-lock.txt
```

## 4. Build بخش های قابل اجرا روی Mac

بخش `rustland-scheduler` ممکن است روی macOS ساخته نشود، چون به کتابخانه ها و APIهای لینوکسی وابسته است. بنابراین برای توسعه مک، فقط پکیج های userspace و simulator را Build کن:

```bash
cargo build \
  -p afs-common \
  -p afs-policy-core \
  -p afs-policy-simulator \
  -p afs-workload-generator \
  -p afs-workload-worker \
  -p afs-fusion-pipeline
```

اجرای تست ها:

```bash
cargo test \
  -p afs-common \
  -p afs-policy-core \
  -p afs-policy-simulator
```

## 5. اجرای شبیه ساز روی Mac

سناریوی Smoke:

```bash
cargo run -p afs-policy-simulator -- \
  --config configs/smoke.toml \
  --scheduler proposed \
  --out-dir results/mac-smoke/proposed
```

سناریوی Light:

```bash
cargo run -p afs-policy-simulator -- \
  --config configs/light.toml \
  --scheduler proposed \
  --out-dir results/mac-light/proposed
```

سناریوی Overload:

```bash
cargo run -p afs-policy-simulator -- \
  --config configs/overload.toml \
  --scheduler proposed \
  --out-dir results/mac-overload/proposed
```

اگر CLI نسخه فعلی تفاوت داشت، دستور زیر را اجرا کن و خروجی را بررسی کن:

```bash
cargo run -p afs-policy-simulator -- --help
```

## 6. تحلیل نتایج روی Mac

پس از تولید CSVها:

```bash
python analysis/analyze_results.py \
  results/mac-smoke/proposed/tasks.csv \
  results/mac-light/proposed/tasks.csv \
  results/mac-overload/proposed/tasks.csv \
  --out results/mac-summary
```

تولید نمودارها:

```bash
python analysis/plot_results.py \
  results/mac-summary/aggregate-summary.csv \
  --out-dir results/mac-summary/plots
```

## 7. اجرای نمونه نتایج آماده

برای اطمینان از درست بودن ابزار تحلیل بدون اجرای شبیه ساز:

```bash
python analysis/analyze_results.py results/sample/*/tasks.csv --out results/sample-analysis
python analysis/plot_results.py results/sample-analysis/aggregate-summary.csv --out-dir results/sample-analysis/plots
```

نتایج موجود در `results/sample` فقط برای تست Pipeline تحلیل هستند و نتیجه پژوهشی واقعی محسوب نمی شوند.

## 8. انتقال نتایج بین Linux و Mac

سناریوی معمول:

1. اجرای واقعی Scheduler و Baselineها روی Linux 6.12.95
2. کپی پوشه `results/real` به Mac
3. تحلیل و رسم نمودار روی Mac
4. تکمیل گزارش و اسلایدها

نمونه کپی با `scp`:

```bash
scp -r user@linux-host:/path/to/ai-fusion-scheduler/results/real ./results/
```

## 9. نکات مربوط به Apple Silicon

- مسیر Homebrew معمولاً `/opt/homebrew` است.
- بعضی پکیج های Python روی Apple Silicon نیاز به wheel سازگار دارند؛ اگر خطا گرفتی ابتدا `pip install --upgrade pip setuptools wheel` را اجرا کن.
- زمان اجرای CPU روی Mac با Linux قابل مقایسه مستقیم نیست؛ فقط برای توسعه و شبیه سازی از آن استفاده کن.
- نتایج نهایی مقاله/گزارش باید از Linux مرجع گرفته شوند.

## 10. چک لیست سریع Mac

```text
[ ] Rust 1.82.0 فعال است
[ ] Python venv ساخته شده است
[ ] requirements-lock.txt نصب شده است
[ ] پکیج های userspace Build می شوند
[ ] تست های policy-core پاس می شوند
[ ] policy-simulator با smoke.toml اجرا می شود
[ ] analysis/analyze_results.py روی sample results اجرا می شود
[ ] نمودارها ساخته می شوند
```
