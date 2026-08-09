# Validation Status

## Package Version

```text
Project package: 0.2.0
Package date: 1405-05-13 / 2026-08-04
```

## Validated in this build environment

- Project directory structure
- Presence of required documentation
- TOML configuration files
- Python analysis scripts with synthetic sample data
- Generation of sample aggregate CSV
- Generation of sample plots
- PowerPoint slide deck generation and render test
- ZIP integrity test
- SHA-256 checksum generation

## Not validated in this build environment

The following require the target Linux system and root privileges:

- Rust compilation with Cargo
- `sched_ext` kernel availability
- `scx_simple` load/unload
- `scx_rustland` load/unload
- `afs-rustland-scheduler` real BPF integration
- Real Dispatch metrics
- `SCHED_DEADLINE` baseline
- PSI readings from `/proc/pressure/cpu`

## Required target validation

Run on Linux 6.12.95:

```bash
./scripts/check_environment.sh
./scripts/collect_system_info.sh results/environment-before.txt
./scripts/build_userspace.sh
./scripts/build_sched_ext.sh
sudo ./scripts/run_smoke_test.sh
```

If the Smoke Test passes, proceed to:

```bash
./scripts/run_simulator_matrix.sh
./scripts/run_ablations.sh
./scripts/run_fusion_case.sh
```

## Sample results warning

`results/sample` contains synthetic data only. These files are included to verify that analysis and plotting work. They are not empirical research results.
