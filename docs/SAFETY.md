# sched_ext safety procedure

1. Start in a VM or on a dedicated lab machine with a recent snapshot.
2. Keep a second root shell or SSH connection open.
3. Do not enable the custom scheduler at boot.
4. Use partial mode and mark only benchmark workers as `SCHED_EXT`.
5. Run `scripts/emergency_stop.sh` in the second shell if the test becomes unresponsive.
6. Keep kernel logs (`journalctl -k -f` or `dmesg -w`) visible.
7. Stop on BPF verifier errors, runnable-task stalls, repeated bounced dispatches or scheduler congestion.
8. Verify `/sys/kernel/sched_ext/state` returns `disabled` after every run.
9. Never collect final data while debugging the environment.

The kernel restores the default scheduler when a sched_ext scheduler exits or errors, but this mechanism is not a substitute for a recovery plan.
