# Secondary evaluation v1 summary

Ablations use the same workload/repetition manifests as the frozen primary full-AFS runs.
Periodic measured methods use the same repetition manifest and the same 2-5 cpuset partition-root execution domain.
Fusion v14 runs the wrapper as the regular user and reinitializes the CPU partition inside the child runner after lib.sh is sourced; scheduler privilege remains confined to the scheduler helper.

## No-aging ablation

| Workload | Variant goodput | Full AFS goodput | Variant accepted miss | Full AFS accepted miss | Variant rejection | Full AFS rejection | Variant P95 ms | Full AFS P95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| overload-bursty | 10.56% | 13.32% | 51.24% | 37.01% | 78.97% | 79.38% | 6357.2 | 7025.8 |
| saturated | 83.04% | 82.17% | 7.72% | 7.01% | 16.69% | 18.05% | 6439.7 | 6186.9 |

## No-application-deadline ablation

| Workload | Variant goodput | Full AFS goodput | Variant accepted miss | Full AFS accepted miss | Variant rejection | Full AFS rejection | Variant P95 ms | Full AFS P95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| overload-bursty | 0.40% | 13.32% | 99.02% | 37.01% | 60.52% | 79.38% | 25495.5 | 7025.8 |
| saturated | 58.70% | 82.17% | 15.54% | 7.01% | 34.29% | 18.05% | 7151.0 | 6186.9 |

## Periodic/sporadic comparison

| Method | Deadline goodput | Accepted miss | Rejection | Completion | P95 response ms |
|---|---:|---:|---:|---:|---:|
| eevdf | 99.33% | 0.67% | 0.00% | 100.00% | 111.0 |
| proposed | 99.87% | 0.13% | 0.00% | 100.00% | 104.2 |
| sched-deadline | 96.53% | 3.47% | 0.00% | 100.00% | 147.8 |

## Fusion case study

- Full-AFS fusion pipeline completed: yes
- Fusion runner privilege: regular user; scheduler-only privileged launch.
