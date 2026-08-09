# Threats to validity

- **Runtime-estimation error:** application runtime can vary with cache state and input. Report estimation MAE and repeat key experiments with ±10%, ±25% and ±50% injected error.
- **Virtualization:** a hypervisor schedules vCPUs independently. Prefer bare metal; otherwise report the hypervisor and treat absolute latency carefully.
- **Thermal and frequency effects:** fix the governor, record temperature/throttling, randomize method order and use cool-down intervals.
- **I/O noise:** isolate benchmark files, flush/retain caches consistently and report whether cold or warm cache is used.
- **Whole-system interference:** partial mode improves safety but changes the experiment population. Clearly state that only benchmark workers use `SCHED_EXT`.
- **Baseline parameterization:** `SCHED_DEADLINE` results depend on defensible runtime/deadline/period values. Do not apply it to scenarios that do not fit its model.
- **Rejecting work:** always pair deadline miss ratio with rejection rate and goodput.
- **Single-node scope:** conclusions apply to node-level CPU scheduling, not cluster placement or network-aware offloading.
