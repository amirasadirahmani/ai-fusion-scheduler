# Target-host build notes

The delivery environment could not compile Rust or load BPF. Perform these
steps on the frozen Linux 6.12.95 lab host.

```bash
./scripts/check_environment.sh
./scripts/collect_system_info.sh results/environment-before.txt
./scripts/freeze_dependencies.sh
./scripts/build_userspace.sh
./scripts/build_sched_ext.sh
sudo ./scripts/run_smoke_test.sh
```

The adapter defaults to `scx_rustland_core = 2.4.13`, `scx_cargo = 1.1.2` and
`libbpf-rs = 0.26.2`. If the frozen host uses a different compatible scx line,
change all pins together and compare the adapter with that release's
`scx_rlfifo` source. The crate exposes an alternate `rustland-init4` feature for
older four-argument `BpfScheduler::init` APIs:

```bash
cargo build --release -p afs-rustland-scheduler \
  --no-default-features --features rustland-init4
```

Never adjust dependencies during the final experiment matrix without rerunning
all methods.
