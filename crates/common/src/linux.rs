use crate::SchedulerMethod;
use std::fs;
use std::io;
use std::path::Path;

pub const SCHED_EXT_POLICY: i32 = 7;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SchedAttr {
    pub size: u32,
    pub sched_policy: u32,
    pub sched_flags: u64,
    pub sched_nice: i32,
    pub sched_priority: u32,
    pub sched_runtime: u64,
    pub sched_deadline: u64,
    pub sched_period: u64,
    pub sched_util_min: u32,
    pub sched_util_max: u32,
}

impl Default for SchedAttr {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            sched_policy: 0,
            sched_flags: 0,
            sched_nice: 0,
            sched_priority: 0,
            sched_runtime: 0,
            sched_deadline: 0,
            sched_period: 0,
            sched_util_min: 0,
            sched_util_max: 1024,
        }
    }
}

pub fn set_sched_ext(pid: i32) -> io::Result<()> {
    let param = libc::sched_param { sched_priority: 0 };
    let rc = unsafe { libc::sched_setscheduler(pid, SCHED_EXT_POLICY, &param) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn set_sched_other(pid: i32) -> io::Result<()> {
    let param = libc::sched_param { sched_priority: 0 };
    let rc = unsafe { libc::sched_setscheduler(pid, libc::SCHED_OTHER, &param) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn set_sched_deadline(
    pid: i32,
    runtime_ns: u64,
    deadline_ns: u64,
    period_ns: u64,
) -> io::Result<()> {
    if runtime_ns == 0 || deadline_ns == 0 || period_ns == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime/deadline/period must be positive",
        ));
    }
    if runtime_ns > deadline_ns || deadline_ns > period_ns {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected runtime <= deadline <= period",
        ));
    }
    let attr = SchedAttr {
        sched_policy: 6, // SCHED_DEADLINE
        sched_runtime: runtime_ns,
        sched_deadline: deadline_ns,
        sched_period: period_ns,
        ..Default::default()
    };
    let rc = unsafe {
        libc::syscall(
            libc::SYS_sched_setattr,
            pid,
            &attr as *const SchedAttr,
            0u32,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn apply_scheduler_policy(
    method: SchedulerMethod,
    pid: i32,
    runtime_ns: u64,
    deadline_ns: Option<u64>,
    period_ns: Option<u64>,
) -> io::Result<()> {
    match method {
        SchedulerMethod::Eevdf | SchedulerMethod::Simulator => set_sched_other(pid),
        SchedulerMethod::Rustland | SchedulerMethod::Proposed => set_sched_ext(pid),
        SchedulerMethod::SchedDeadline => {
            let deadline = deadline_ns.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "deadline is required")
            })?;
            let period = period_ns.unwrap_or(deadline);
            set_sched_deadline(pid, runtime_ns, deadline, period)
        }
    }
}

pub fn signal(pid: i32, sig: i32) -> io::Result<()> {
    let rc = unsafe { libc::kill(pid, sig) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn proc_process_cpu_time_ns(pid: i32) -> io::Result<u64> {
    let text = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    // comm is parenthesized and may contain spaces; split after the final ')'.
    let close = text
        .rfind(')')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid /proc stat format"))?;
    let rest = text
        .get(close + 2..)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid /proc stat fields"))?;
    // Fields after comm start at field 3. utime=14 and stime=15 => indexes 11 and 12.
    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.len() <= 12 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not enough /proc stat fields",
        ));
    }
    let utime: u64 = fields[11]
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid utime"))?;
    let stime: u64 = fields[12]
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid stime"))?;
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if ticks <= 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((utime.saturating_add(stime)).saturating_mul(1_000_000_000) / ticks as u64)
}

pub fn sched_ext_state() -> io::Result<String> {
    fs::read_to_string("/sys/kernel/sched_ext/state").map(|s| s.trim().to_string())
}

pub fn ensure_dir(path: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(path)
}
