use std::io;
use std::time::Duration;

/// Return CLOCK_MONOTONIC in nanoseconds.
pub fn monotonic_ns() -> io::Result<u64> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    let sec = u64::try_from(ts.tv_sec).unwrap_or(0);
    let nsec = u64::try_from(ts.tv_nsec).unwrap_or(0);
    Ok(sec.saturating_mul(1_000_000_000).saturating_add(nsec))
}

/// Return CPU time consumed by the current process in nanoseconds.
pub fn process_cpu_time_ns() -> io::Result<u64> {
    clock_ns(libc::CLOCK_PROCESS_CPUTIME_ID)
}

/// Return CPU time consumed by the current thread in nanoseconds.
pub fn thread_cpu_time_ns() -> io::Result<u64> {
    clock_ns(libc::CLOCK_THREAD_CPUTIME_ID)
}

fn clock_ns(clock_id: libc::clockid_t) -> io::Result<u64> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let rc = unsafe { libc::clock_gettime(clock_id, &mut ts) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    let sec = u64::try_from(ts.tv_sec).unwrap_or(0);
    let nsec = u64::try_from(ts.tv_nsec).unwrap_or(0);
    Ok(sec.saturating_mul(1_000_000_000).saturating_add(nsec))
}

pub fn ms_to_ns(ms: u64) -> u64 {
    ms.saturating_mul(1_000_000)
}

pub fn ns_to_ms_f64(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

pub fn sleep_ns(ns: u64) {
    std::thread::sleep(Duration::from_nanos(ns));
}
