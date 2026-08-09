#!/usr/bin/env python3
from __future__ import annotations
import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
errors: list[str] = []

required = [
    "Cargo.toml",
    "README.md",
    "configs/smoke.toml",
    "configs/fusion-pipeline.toml",
    "crates/rustland-scheduler/src/main.rs",
    "analysis/analyze_results.py",
    "scripts/run_experiment.sh",
    "scripts/test_analysis.py",
    "docs/QUICK_START_FA.md",
    "docs/final-proposal-fa.pdf",
    "PROJECT_CHECKLIST.md",
]
for rel in required:
    if not (ROOT / rel).exists():
        errors.append(f"missing required file: {rel}")

for path in sorted((ROOT / "configs").glob("*.toml")):
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
        if "experiment" not in data:
            errors.append(f"{path.name}: missing [experiment]")
        if not data.get("workloads") and not data.get("pipeline"):
            errors.append(f"{path.name}: needs workloads or pipeline")
        policy = data.get("policy", {})
        alpha = policy.get("runtime_ewma_alpha")
        if not isinstance(alpha, (int, float)) or not (0 < float(alpha) <= 1):
            errors.append(f"{path.name}: invalid policy.runtime_ewma_alpha")
    except Exception as exc:
        errors.append(f"{path.name}: TOML parse failed: {exc}")

for path in ROOT.rglob("*.json"):
    if any(part in {"results", "run", "target"} for part in path.parts):
        continue
    try:
        json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        errors.append(f"{path.relative_to(ROOT)}: JSON parse failed: {exc}")

for path in ROOT.rglob("*.rs"):
    text = path.read_text(encoding="utf-8")
    if "TODO_UNIMPLEMENTED" in text or "unimplemented!()" in text:
        errors.append(f"{path.relative_to(ROOT)}: unresolved implementation marker")
    if "Some(decision.reason)" in text:
        errors.append(f"{path.relative_to(ROOT)}: likely String/&str type mismatch")

for path in sorted((ROOT / "scripts").glob("*.sh")):
    if not (path.stat().st_mode & 0o111):
        errors.append(f"{path.relative_to(ROOT)}: script is not executable")

if errors:
    print("static validation failed:", file=sys.stderr)
    for error in errors:
        print(f"  - {error}", file=sys.stderr)
    raise SystemExit(1)
print("static validation passed")
