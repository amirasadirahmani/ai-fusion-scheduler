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
             1_000_000_000, "", "", 1_150_000_000, 100_000_000, "", "", "", "", "overload"],
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
        (run / "run-provenance.txt").write_text(
            "method=proposed-no-admission\n",
            encoding="utf-8",
        )
        (run / "workload-cpu-pressure-final.txt").write_text(
            "some avg10=12.50 avg60=5.00 avg300=1.00 total=100000\n"
            "full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n",
            encoding="utf-8",
        )

        summary = module.summarize_file(run / "tasks.csv")
        assert summary["scheduler"] == "proposed-no-admission"
        assert summary["scheduler_impl"] == "proposed"
        assert summary["offered_deadline_tasks"] == 3
        assert summary["deadline_successes"] == 1
        assert summary["deadline_goodput"] == 1 / 3
        assert summary["accepted_deadline_tasks"] == 2
        assert summary["accepted_deadline_misses"] == 1
        assert summary["accepted_miss_rate"] == 0.5
        assert summary["p50_response_ms"] == 200.0
        assert summary["workload_psi_some_avg10_final"] == 12.5
        assert summary["workload_psi_some_total_us"] == 100000
        assert summary["workload_psi_some_stall_fraction"] == 0.2

        assert summary["critical_offered_tasks"] == 1
        assert summary["critical_deadline_goodput"] == 1.0
        assert summary["critical_accepted_miss_rate"] == 0.0
        assert summary["critical_p50_response_ms"] == 100.0

        assert summary["interactive_offered_tasks"] == 1
        assert summary["interactive_deadline_goodput"] == 0.0
        assert summary["interactive_accepted_miss_rate"] == 1.0
        assert summary["interactive_p50_response_ms"] == 300.0

        assert summary["batch_offered_tasks"] == 1
        assert summary["batch_rejection_rate"] == 1.0
        assert summary["batch_deadline_goodput"] == 0.0
        assert summary["batch_accepted_deadline_tasks"] == 0
        assert summary["batch_accepted_miss_rate"] == ""

        assert summary["deadline_miss_ratio"] == 0.5
        assert summary["p95_response_ms"] == 290.0
        assert summary["admission_rate"] == 2 / 3
        assert summary["average_tasks_per_notify_cycle"] == 3.0
        assert summary["average_decision_wall_us"] == 100.0
        assert summary["core_failed_dispatches"] == 1
        agg = module.aggregate([summary])
        assert len(agg) == 1
        assert agg[0]["scheduler"] == "proposed-no-admission"
        assert agg[0]["runs"] == 1
        assert agg[0]["deadline_goodput_mean"] == 1 / 3
        assert agg[0]["p50_response_ms_mean"] == 200.0
        assert agg[0]["workload_psi_some_stall_fraction_mean"] == 0.2
        assert agg[0]["critical_deadline_goodput_mean"] == 1.0
        assert agg[0]["interactive_deadline_goodput_mean"] == 0.0
        assert agg[0]["batch_rejection_rate_mean"] == 1.0
        assert agg[0]["accepted_miss_rate_mean"] == 0.5

    print("analysis self-test passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
