#!/usr/bin/env python3
"""Aggregate task-level CSV files produced by AI Fusion Scheduler.

The script deliberately separates admission rate from deadline miss ratio so a
policy cannot appear successful merely by rejecting most work.
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Iterable


def parse_int(value: str | None) -> int | None:
    if value is None or value == "":
        return None
    try:
        return int(value)
    except ValueError:
        return None


def parse_bool(value: str | None) -> bool | None:
    if value is None or value == "":
        return None
    return value.strip().lower() in {"true", "1", "yes"}


def percentile(values: list[float], p: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    position = (len(ordered) - 1) * p
    lo = math.floor(position)
    hi = math.ceil(position)
    if lo == hi:
        return ordered[lo]
    fraction = position - lo
    return ordered[lo] * (1.0 - fraction) + ordered[hi] * fraction


def jain(values: Iterable[float]) -> float | None:
    xs = [x for x in values if x >= 0.0]
    if not xs:
        return None
    denom = len(xs) * sum(x * x for x in xs)
    if denom == 0.0:
        return None
    total = sum(xs)
    return total * total / denom


def read_json_if_present(path: Path) -> dict:
    if not path.exists():
        return {}
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else {}
    except (OSError, json.JSONDecodeError):
        return {}


def find_run_input(tasks_csv: Path) -> dict:
    return read_json_if_present(tasks_csv.parent / "run-input.json")


def find_run_summary(tasks_csv: Path) -> dict:
    return read_json_if_present(tasks_csv.parent / "summary.json")


def find_scheduler_stats(tasks_csv: Path) -> dict:
    # The proposed adapter writes scheduler-stats.json. Keep a compatibility
    # fallback for early development runs that used the longer filename.
    for name in ("scheduler-stats.json", "proposed-scheduler-stats.json"):
        value = read_json_if_present(tasks_csv.parent / name)
        if value:
            return value
    return {}


def value_or_blank(value: float | int | None) -> float | int | str:
    return "" if value is None else value


def summarize_file(path: Path) -> dict:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if not rows:
        raise ValueError(f"empty CSV: {path}")

    run_input = find_run_input(path)
    run_summary = find_run_summary(path)
    scheduler_stats = find_scheduler_stats(path)
    core_stats = scheduler_stats.get("core") if isinstance(scheduler_stats.get("core"), dict) else {}
    cpus = int(run_input.get("cpus") or 1)
    experiment = rows[0].get("experiment", "")
    scheduler = rows[0].get("scheduler", "")
    repetition = int(rows[0].get("repetition") or 0)

    total = len(rows)
    rejected = [r for r in rows if r.get("admission") == "reject"]
    delayed = [r for r in rows if r.get("admission") == "delay"]
    admitted = [r for r in rows if r.get("admission") != "reject"]
    completed = [
        r
        for r in admitted
        if parse_int(r.get("completion_time_ns")) is not None
        and parse_int(r.get("exit_code")) == 0
    ]
    failed = [
        r for r in admitted if parse_int(r.get("exit_code")) not in (None, 0)
    ]
    deadline_completed = [
        r for r in completed if parse_int(r.get("absolute_deadline_ns")) is not None
    ]
    missed = [r for r in deadline_completed if parse_bool(r.get("deadline_missed")) is True]
    timely = [r for r in deadline_completed if parse_bool(r.get("deadline_missed")) is False]

    releases = [parse_int(r.get("release_time_ns")) for r in rows]
    completions = [parse_int(r.get("completion_time_ns")) for r in completed]
    valid_releases = [v for v in releases if v is not None]
    valid_completions = [v for v in completions if v is not None]
    start_ns = min(valid_releases) if valid_releases else 0
    end_ns = max(valid_completions) if valid_completions else start_ns
    summary_started = parse_int(str(run_summary.get("started_ns", "")))
    summary_finished = parse_int(str(run_summary.get("finished_ns", "")))
    if summary_started is not None and summary_finished is not None and summary_finished >= summary_started:
        duration_s = max((summary_finished - summary_started) / 1e9, 1e-9)
    else:
        duration_s = max((end_ns - start_ns) / 1e9, 1e-9)

    responses_ms = [
        value / 1e6
        for value in (parse_int(r.get("response_time_ns")) for r in completed)
        if value is not None
    ]
    actual_cpu_ns = sum(
        value
        for value in (parse_int(r.get("actual_cpu_time_ns")) for r in completed)
        if value is not None
    )
    cpu_utilization = actual_cpu_ns / (duration_s * 1e9 * max(cpus, 1))

    class_demand: dict[str, float] = defaultdict(float)
    class_service: dict[str, float] = defaultdict(float)
    for row in admitted:
        cls = row.get("workload_class", "unknown")
        class_demand[cls] += float(parse_int(row.get("estimated_runtime_ns")) or 0)
    for row in completed:
        cls = row.get("workload_class", "unknown")
        class_service[cls] += float(parse_int(row.get("actual_cpu_time_ns")) or 0)
    service_ratios = [
        min(class_service[cls] / demand, 1.0)
        for cls, demand in class_demand.items()
        if demand > 0
    ]

    metadata_cache_hits = parse_int(str(scheduler_stats.get("metadata_cache_hits", "")))
    metadata_registry_reads = parse_int(str(scheduler_stats.get("metadata_registry_reads", "")))
    metadata_misses = parse_int(str(scheduler_stats.get("metadata_misses", "")))
    local_dispatches = parse_int(str(scheduler_stats.get("local_dispatches", "")))
    dispatch_attempts = parse_int(str(scheduler_stats.get("dispatch_attempts", "")))
    tasks_dequeued = parse_int(str(scheduler_stats.get("tasks_dequeued", "")))
    notify_cycles = parse_int(str(scheduler_stats.get("notify_cycles", "")))
    tasks_in_notify_cycles = parse_int(str(scheduler_stats.get("tasks_in_notify_cycles", "")))
    scheduler_cpu_time_ns = parse_int(str(scheduler_stats.get("scheduler_cpu_time_ns", "")))
    total_decision_wall_ns = parse_int(str(scheduler_stats.get("total_decision_wall_ns", "")))
    max_decision_wall_ns = parse_int(str(scheduler_stats.get("max_decision_wall_ns", "")))
    total_cycle_wall_ns = parse_int(str(scheduler_stats.get("total_cycle_wall_ns", "")))
    max_cycle_wall_ns = parse_int(str(scheduler_stats.get("max_cycle_wall_ns", "")))
    average_batch = (
        tasks_in_notify_cycles / notify_cycles
        if tasks_in_notify_cycles is not None and notify_cycles not in (None, 0)
        else None
    )
    average_decision_wall_us = (
        total_decision_wall_ns / notify_cycles / 1e3
        if total_decision_wall_ns is not None and notify_cycles not in (None, 0)
        else None
    )
    decision_wall_ns_per_dispatch = (
        total_decision_wall_ns / dispatch_attempts
        if total_decision_wall_ns is not None and dispatch_attempts not in (None, 0)
        else None
    )
    average_cycle_wall_us = (
        total_cycle_wall_ns / notify_cycles / 1e3
        if total_cycle_wall_ns is not None and notify_cycles not in (None, 0)
        else None
    )
    scheduler_cpu_fraction = (
        scheduler_cpu_time_ns / (duration_s * 1e9)
        if scheduler_cpu_time_ns is not None
        else None
    )

    return {
        "source_csv": str(path),
        "experiment": experiment,
        "repetition": repetition,
        "scheduler": scheduler,
        "cpus": cpus,
        "total_tasks": total,
        "admitted_tasks": len(admitted),
        "delayed_tasks": len(delayed),
        "rejected_tasks": len(rejected),
        "completed_tasks": len(completed),
        "failed_tasks": len(failed),
        "deadline_completed_tasks": len(deadline_completed),
        "deadline_misses": len(missed),
        "admission_rate": len(admitted) / total if total else 0.0,
        "delay_rate": len(delayed) / total if total else 0.0,
        "rejection_rate": len(rejected) / total if total else 0.0,
        "completion_rate": len(completed) / total if total else 0.0,
        "deadline_miss_ratio": len(missed) / len(deadline_completed)
        if deadline_completed
        else "",
        "mean_response_ms": statistics.fmean(responses_ms) if responses_ms else "",
        "p95_response_ms": value_or_blank(percentile(responses_ms, 0.95)),
        "throughput_tasks_s": len(completed) / duration_s,
        "goodput_deadline_tasks_s": len(timely) / duration_s,
        "cpu_utilization": cpu_utilization,
        "jain_class_service_fairness": value_or_blank(jain(service_ratios)),
        "duration_s": duration_s,
        "scheduler_cpu_time_ms": value_or_blank(
            scheduler_cpu_time_ns / 1e6 if scheduler_cpu_time_ns is not None else None
        ),
        "scheduler_cpu_fraction": value_or_blank(scheduler_cpu_fraction),
        "metadata_cache_hits": value_or_blank(metadata_cache_hits),
        "metadata_registry_reads": value_or_blank(metadata_registry_reads),
        "metadata_misses": value_or_blank(metadata_misses),
        "local_dispatches": value_or_blank(local_dispatches),
        "dispatch_attempts": value_or_blank(dispatch_attempts),
        "tasks_dequeued": value_or_blank(tasks_dequeued),
        "notify_cycles": value_or_blank(notify_cycles),
        "average_tasks_per_notify_cycle": value_or_blank(average_batch),
        "average_decision_wall_us": value_or_blank(average_decision_wall_us),
        "decision_wall_ns_per_dispatch": value_or_blank(decision_wall_ns_per_dispatch),
        "max_decision_wall_us": value_or_blank(
            max_decision_wall_ns / 1e3 if max_decision_wall_ns is not None else None
        ),
        "average_cycle_wall_us": value_or_blank(average_cycle_wall_us),
        "max_cycle_wall_us": value_or_blank(
            max_cycle_wall_ns / 1e3 if max_cycle_wall_ns is not None else None
        ),
        "core_user_dispatches": value_or_blank(parse_int(str(core_stats.get("user_dispatches", "")))),
        "core_kernel_dispatches": value_or_blank(parse_int(str(core_stats.get("kernel_dispatches", "")))),
        "core_cancelled_dispatches": value_or_blank(parse_int(str(core_stats.get("cancelled_dispatches", "")))),
        "core_bounced_dispatches": value_or_blank(parse_int(str(core_stats.get("bounced_dispatches", "")))),
        "core_failed_dispatches": value_or_blank(parse_int(str(core_stats.get("failed_dispatches", "")))),
        "core_congestion_events": value_or_blank(parse_int(str(core_stats.get("congestion_events", "")))),
    }


def mean_ci95(values: list[float]) -> tuple[float | str, float | str]:
    if not values:
        return "", ""
    mean = statistics.fmean(values)
    if len(values) < 2:
        return mean, ""
    sd = statistics.stdev(values)
    return mean, 1.96 * sd / math.sqrt(len(values))


def aggregate(rows: list[dict]) -> list[dict]:
    groups: dict[tuple[str, str], list[dict]] = defaultdict(list)
    for row in rows:
        groups[(str(row["experiment"]), str(row["scheduler"]))].append(row)

    metrics = [
        "admission_rate",
        "rejection_rate",
        "deadline_miss_ratio",
        "mean_response_ms",
        "p95_response_ms",
        "throughput_tasks_s",
        "goodput_deadline_tasks_s",
        "cpu_utilization",
        "jain_class_service_fairness",
        "scheduler_cpu_time_ms",
        "scheduler_cpu_fraction",
        "metadata_cache_hits",
        "metadata_registry_reads",
        "metadata_misses",
        "local_dispatches",
        "dispatch_attempts",
        "tasks_dequeued",
        "notify_cycles",
        "average_tasks_per_notify_cycle",
        "average_decision_wall_us",
        "decision_wall_ns_per_dispatch",
        "max_decision_wall_us",
        "average_cycle_wall_us",
        "max_cycle_wall_us",
        "core_user_dispatches",
        "core_kernel_dispatches",
        "core_cancelled_dispatches",
        "core_bounced_dispatches",
        "core_failed_dispatches",
        "core_congestion_events",
    ]
    output: list[dict] = []
    for (experiment, scheduler), items in sorted(groups.items()):
        row: dict[str, object] = {
            "experiment": experiment,
            "scheduler": scheduler,
            "runs": len(items),
        }
        for metric in metrics:
            values = [
                float(item[metric])
                for item in items
                if item.get(metric) not in (None, "")
            ]
            mean, ci = mean_ci95(values)
            row[f"{metric}_mean"] = mean
            row[f"{metric}_ci95"] = ci
        output.append(row)
    return output


def write_csv(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if not rows:
        path.write_text("", encoding="utf-8")
        return
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results_dir", type=Path)
    parser.add_argument("--out", type=Path, default=Path("results/run-summary.csv"))
    parser.add_argument(
        "--aggregate-out",
        type=Path,
        default=Path("results/aggregate-summary.csv"),
    )
    args = parser.parse_args()

    files = sorted(args.results_dir.rglob("tasks.csv"))
    if not files:
        parser.error(f"no tasks.csv files found below {args.results_dir}")
    rows = [summarize_file(path) for path in files]
    write_csv(args.out, rows)
    write_csv(args.aggregate_out, aggregate(rows))
    print(f"analyzed {len(files)} runs -> {args.out} and {args.aggregate_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
