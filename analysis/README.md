# Analysis workflow

```bash
python3 analysis/analyze_results.py results \
  --out results/run-summary.csv \
  --aggregate-out results/aggregate-summary.csv
python3 analysis/plot_results.py results/aggregate-summary.csv --out-dir results/plots
python3 analysis/make_markdown_table.py results/aggregate-summary.csv
```

`analyze_results.py` uses only the Python standard library. Plotting requires
`matplotlib`. Deadline miss ratio is calculated only among admitted, completed
jobs that have an application deadline. Admission/rejection rates and goodput
are always reported beside it to prevent rejection-heavy policies from looking
artificially successful.
