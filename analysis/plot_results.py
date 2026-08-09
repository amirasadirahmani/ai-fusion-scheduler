#!/usr/bin/env python3
"""Create separate comparison plots from aggregate-summary.csv."""
from __future__ import annotations

import argparse
import csv
from collections import defaultdict
from pathlib import Path


def load_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def plot_metric(rows: list[dict[str, str]], metric: str, ylabel: str, out: Path) -> None:
    try:
        import matplotlib.pyplot as plt
    except ImportError as exc:
        raise SystemExit("matplotlib is required: python3 -m pip install matplotlib") from exc

    by_experiment: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        if row.get(f"{metric}_mean", "") != "":
            by_experiment[row["experiment"]].append(row)
    if not by_experiment:
        return

    experiments = sorted(by_experiment)
    schedulers = sorted({r["scheduler"] for r in rows})
    x = list(range(len(experiments)))
    width = 0.8 / max(len(schedulers), 1)

    fig, ax = plt.subplots(figsize=(max(8, len(experiments) * 2.0), 5.2))
    for scheduler_index, scheduler in enumerate(schedulers):
        values = []
        errors = []
        positions = []
        for experiment_index, experiment in enumerate(experiments):
            match = next(
                (r for r in by_experiment[experiment] if r["scheduler"] == scheduler),
                None,
            )
            if match is None or match.get(f"{metric}_mean", "") == "":
                continue
            positions.append(experiment_index + (scheduler_index - (len(schedulers)-1)/2) * width)
            values.append(float(match[f"{metric}_mean"]))
            ci = match.get(f"{metric}_ci95", "")
            errors.append(float(ci) if ci else 0.0)
        if positions:
            ax.bar(positions, values, width=width, yerr=errors, capsize=3, label=scheduler)

    ax.set_xticks(x, experiments, rotation=20, ha="right")
    ax.set_ylabel(ylabel)
    ax.set_xlabel("Experiment")
    ax.legend()
    ax.grid(axis="y", alpha=0.25)
    fig.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=180)
    plt.close(fig)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("summary_csv", type=Path)
    parser.add_argument("--out-dir", type=Path, default=Path("results/plots"))
    args = parser.parse_args()
    rows = load_rows(args.summary_csv)
    plots = [
        ("deadline_miss_ratio", "Deadline miss ratio", "deadline-miss-ratio.png"),
        ("p95_response_ms", "P95 response time (ms)", "p95-response.png"),
        ("goodput_deadline_tasks_s", "Deadline goodput (tasks/s)", "goodput.png"),
        ("jain_class_service_fairness", "Jain class-service fairness", "fairness.png"),
        ("rejection_rate", "Rejection rate", "rejection-rate.png"),
        ("cpu_utilization", "Normalized CPU utilization", "cpu-utilization.png"),
        ("scheduler_cpu_fraction", "Scheduler CPU fraction", "scheduler-cpu-fraction.png"),
        ("decision_wall_ns_per_dispatch", "Decision wall time per dispatch (ns)", "decision-overhead.png"),
    ]
    for metric, ylabel, filename in plots:
        plot_metric(rows, metric, ylabel, args.out_dir / filename)
    print(f"plots written below {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
