#!/usr/bin/env python3
from __future__ import annotations

import csv
import importlib.util
import json
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "afs_analyze_results", ROOT / "analysis" / "analyze_results.py"
)
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


def main() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        run = Path(tmp) / "run"
        run.mkdir()
        fields = [
            "experiment", "repetition", "scheduler", "task_id", "process_id",
            "workflow_id", "stage_id", "task_name", "workload_class", "kind",
            "admission", "delayed_ns", "release_time_ns", "start_time_ns",
            "completion_time_ns", "absolute_deadline_ns", "estimated_runtime_ns",
            "actual_cpu_time_ns", "response_time_ns", "deadline_missed", "exit_code", "error",
        ]
        rows = [
            ["smoke", 0, "proposed", 1, 100, 0, 0, "t1", "critical", "cpu", "admit", 0,
             1_000_000_000, 1_000_000_000, 1_100_000_000, 1_200_000_000,
             100_000_000, 90_000_000, 100_000_000, "false", 0, ""],
            ["smoke", 0, "proposed", 2, 101, 0, 0, "t2", "interactive", "cpu", "delay", 10_000_000,
             1_000_000_000, 1_020_000_000, 1_300_000_000, 1_250_000_000,
             200_000_000, 180_000_000, 300_000_000, "true", 0, ""],
            ["smoke", 0, "proposed", 3, -1, 0, 0, "t3", "batch", "cpu", "reject", 0,
             1_000_000_000, "", "", "", 100_000_000, "", "", "", "", "overload"],
        ]
        with (run / "tasks.csv").open("w", newline="", encoding="utf-8") as handle:
            writer = csv.writer(handle)
            writer.writerow(fields)
            writer.writerows(rows)
        (run / "run-input.json").write_text(json.dumps({"cpus": 2}), encoding="utf-8")
        (run / "summary.json").write_text(
            json.dumps({"started_ns": 1_000_000_000, "finished_ns": 1_500_000_000}),
            encoding="utf-8",
        )
        (run / "scheduler-stats.json").write_text(
            json.dumps({
                "local_dispatches": 12,
                "dispatch_attempts": 13,
                "tasks_dequeued": 14,
                "tasks_in_notify_cycles": 12,
                "notify_cycles": 4,
                "scheduler_cpu_time_ns": 50_000_000,
                "total_decision_wall_ns": 400_000,
                "max_decision_wall_ns": 200_000,
                "total_cycle_wall_ns": 100_000_000,
                "max_cycle_wall_ns": 50_000_000,
                "core": {"user_dispatches": 12, "failed_dispatches": 1},
            }),
            encoding="utf-8",
        )
        summary = module.summarize_file(run / "tasks.csv")
        assert summary["deadline_miss_ratio"] == 0.5
        assert summary["p95_response_ms"] == 290.0
        assert summary["admission_rate"] == 2 / 3
        assert summary["average_tasks_per_notify_cycle"] == 3.0
        assert summary["average_decision_wall_us"] == 100.0
        assert summary["core_failed_dispatches"] == 1
    print("analysis self-test passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
