#!/usr/bin/env python3
"""Render the main aggregate metrics as a Markdown table for the final report."""
from __future__ import annotations

import argparse
import csv
from pathlib import Path


def fmt(value: str, digits: int = 3) -> str:
    if value == "":
        return "N/A"
    return f"{float(value):.{digits}f}"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("aggregate_csv", type=Path)
    parser.add_argument("--out", type=Path, default=Path("results/comparison-table.md"))
    args = parser.parse_args()
    with args.aggregate_csv.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    lines = [
        "| Experiment | Scheduler | Runs | DMR | P95 ms | Goodput/s | Reject | Fairness |",
        "|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row in rows:
        lines.append(
            "| {experiment} | {scheduler} | {runs} | {dmr} | {p95} | {goodput} | {reject} | {fair} |".format(
                experiment=row["experiment"],
                scheduler=row["scheduler"],
                runs=row["runs"],
                dmr=fmt(row.get("deadline_miss_ratio_mean", "")),
                p95=fmt(row.get("p95_response_ms_mean", ""), 1),
                goodput=fmt(row.get("goodput_deadline_tasks_s_mean", ""), 2),
                reject=fmt(row.get("rejection_rate_mean", "")),
                fair=fmt(row.get("jain_class_service_fairness_mean", "")),
            )
        )
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(args.out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
