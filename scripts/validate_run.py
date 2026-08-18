#!/usr/bin/env python3

import argparse
import csv
import hashlib
import json
from pathlib import Path


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def fail(message: str) -> None:
    raise SystemExit(f"validation error: {message}")


parser = argparse.ArgumentParser()
parser.add_argument("output_dir", type=Path)
parser.add_argument("--manifest", type=Path)
parser.add_argument("--expected-method")
args = parser.parse_args()

out = args.output_dir
summary_path = out / "summary.json"
tasks_path = out / "tasks.csv"

if not summary_path.is_file():
    fail(f"missing {summary_path}")

if not tasks_path.is_file():
    fail(f"missing {tasks_path}")

summary = json.loads(summary_path.read_text())

with tasks_path.open(newline="") as f:
    rows = list(csv.DictReader(f))

required = {
    "total_tasks",
    "completed",
    "rejected",
    "failed",
    "deadline_misses",
    "scheduler",
}

missing = required.difference(summary)
if missing:
    fail(f"summary missing fields: {sorted(missing)}")

total = int(summary["total_tasks"])
completed = int(summary["completed"])
rejected = int(summary["rejected"])
failed = int(summary["failed"])
misses = int(summary["deadline_misses"])

if len(rows) != total:
    fail(f"tasks.csv rows={len(rows)} but total_tasks={total}")

if completed + rejected + failed != total:
    fail(
        "completed + rejected + failed != total_tasks: "
        f"{completed}+{rejected}+{failed}!={total}"
    )

if failed != 0:
    fail(f"worker/process failures detected: {failed}")

if misses < 0 or misses > completed:
    fail(f"invalid deadline_misses={misses} for completed={completed}")

if args.expected_method and summary["scheduler"] != args.expected_method:
    fail(
        f"scheduler mismatch: expected={args.expected_method} "
        f"actual={summary['scheduler']}"
    )

task_ids = [int(row["task_id"]) for row in rows]

if len(task_ids) != len(set(task_ids)):
    fail("duplicate task_id in tasks.csv")

manifest_sha = None
manifest_count = None

if args.manifest:
    manifest = args.manifest.resolve()

    if not manifest.is_file():
        fail(f"manifest does not exist: {manifest}")

    plans = json.loads(manifest.read_text())

    if not isinstance(plans, list):
        fail("manifest root is not a list")

    manifest_count = len(plans)
    manifest_ids = [int(plan["task_id"]) for plan in plans]

    if manifest_count != total:
        fail(
            f"manifest tasks={manifest_count} "
            f"but summary total_tasks={total}"
        )

    if sorted(manifest_ids) != sorted(task_ids):
        fail("task IDs in manifest and tasks.csv differ")

    manifest_sha = sha256(manifest)

validation = {
    "status": "ok",
    "scheduler": summary["scheduler"],
    "total_tasks": total,
    "completed": completed,
    "rejected": rejected,
    "failed": failed,
    "deadline_misses": misses,
    "manifest_tasks": manifest_count,
    "manifest_sha256": manifest_sha,
}

(out / "run-validation.json").write_text(
    json.dumps(validation, indent=2) + "\n"
)

print(
    "VALID "
    f"method={summary['scheduler']} "
    f"tasks={total} "
    f"completed={completed} "
    f"rejected={rejected} "
    f"failed={failed} "
    f"misses={misses}"
)
