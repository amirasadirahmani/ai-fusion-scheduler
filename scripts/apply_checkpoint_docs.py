#!/usr/bin/env python3
from pathlib import Path
import shutil
import sys

ROOT = Path.cwd()
CHECKPOINT = "2026-08-09"
START = f"<!-- CHECKPOINT-{CHECKPOINT}:START -->"
END = f"<!-- CHECKPOINT-{CHECKPOINT}:END -->"
bundle_root = Path(__file__).resolve().parent.parent

required = ["Cargo.toml", "crates"]
missing = [p for p in required if not (ROOT / p).exists()]
if missing:
    print("ERROR: run this script from the ai-fusion-scheduler repository root.")
    print("Missing:", ", ".join(missing))
    sys.exit(1)

sources = {
    "DECISIONS.md": bundle_root / "snippets" / "DECISIONS_CHECKPOINT.md",
    "ENVIRONMENT.md": bundle_root / "snippets" / "ENVIRONMENT_CHECKPOINT.md",
    "CHANGELOG.md": bundle_root / "snippets" / "CHANGELOG_CHECKPOINT.md",
    "PROJECT_CHECKLIST.md": bundle_root / "snippets" / "PROJECT_CHECKLIST_CHECKPOINT.md",
}

docs_dir = ROOT / "docs"
docs_dir.mkdir(parents=True, exist_ok=True)

report_src = bundle_root / "docs" / "PROJECT_TRANSFER_REPORT.md"
report_dst = docs_dir / "PROJECT_TRANSFER_REPORT.md"

if report_dst.exists():
    backup = report_dst.with_suffix(report_dst.suffix + f".pre-checkpoint-{CHECKPOINT}")
    if not backup.exists():
        shutil.copy2(report_dst, backup)
        print(f"backup: {backup}")

shutil.copy2(report_src, report_dst)
print(f"updated: {report_dst}")

def upsert_block(target: Path, block: str):
    if target.exists():
        original = target.read_text(encoding="utf-8")
        backup = target.with_suffix(target.suffix + f".pre-checkpoint-{CHECKPOINT}")
        if not backup.exists():
            shutil.copy2(target, backup)
            print(f"backup: {backup}")
    else:
        original = ""

    if START in original and END in original:
        before, rest = original.split(START, 1)
        _, after = rest.split(END, 1)
        updated = before.rstrip() + "\n\n" + block.strip() + "\n" + after.lstrip()
    else:
        updated = original.rstrip()
        if updated:
            updated += "\n\n"
        updated += block.strip() + "\n"

    target.write_text(updated, encoding="utf-8")
    print(f"updated: {target}")

for target_name, source in sources.items():
    upsert_block(ROOT / target_name, source.read_text(encoding="utf-8"))

print()
print("Checkpoint documentation applied.")
print("Next:")
print("  bash scripts/capture_reproducibility_checkpoint.sh")
print("  git status")
print("  git diff")
