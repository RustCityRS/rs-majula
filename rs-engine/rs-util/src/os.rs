#[cfg(target_os = "linux")]
use std::ffi::c_ulong;
#[cfg(windows)]
use std::ffi::c_void;
use std::io;
#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};

/// The most files a server asks to keep open: room for every connection cap.
#[cfg(target_os = "linux")]
const MAX_OPEN_FILES: u64 = 1 << 16;

/// Raises the open-file limit as far as the hard limit allows, up to
/// [`MAX_OPEN_FILES`]: systemd's default of 1024 is below the connection caps.
#[cfg(target_os = "linux")]
pub fn raise_open_files() {
    const RLIMIT_NOFILE: i32 = 7;
    #[repr(C)]
    struct Rlimit {
        cur: u64,
        max: u64,
    }
    unsafe extern "C" {
        fn getrlimit(resource: i32, rlim: *mut Rlimit) -> i32;
        fn setrlimit(resource: i32, rlim: *const Rlimit) -> i32;
    }
    let mut limit = Rlimit { cur: 0, max: 0 };
    if unsafe { getrlimit(RLIMIT_NOFILE, &mut limit) } != 0 {
        return;
    }
    let want = limit.max.min(MAX_OPEN_FILES);
    if limit.cur < want {
        limit.cur = want;
        unsafe { setrlimit(RLIMIT_NOFILE, &limit) };
    }
}

#[cfg(not(target_os = "linux"))]
pub fn raise_open_files() {}

/// Turns transparent huge pages off for this process.
#[cfg(target_os = "linux")]
pub fn disable_huge_pages() {
    const PR_SET_THP_DISABLE: i32 = 41;
    unsafe extern "C" {
        fn prctl(option: i32, ...) -> i32;
    }
    unsafe { prctl(PR_SET_THP_DISABLE, 1, 0, 0, 0) };
}

#[cfg(not(target_os = "linux"))]
pub fn disable_huge_pages() {}

/// The realtime priority [`raise_thread_priority`] asks for on Linux: above
/// every normal thread, just under the kernel's interrupt threads (50).
#[cfg(target_os = "linux")]
const REALTIME_PRIORITY: i32 = 49;

/// Puts the calling thread ahead of every normal thread on the machine.
///
/// On Linux it takes the realtime `SCHED_FIFO` policy at
/// [`REALTIME_PRIORITY`], which needs root, `CAP_SYS_NICE` or an
/// `RLIMIT_RTPRIO` that high (systemd's `LimitRTPRIO=`). The kernel still
/// keeps 5% of each core for normal threads (`kernel.sched_rt_runtime_us`).
/// Threads it starts inherit the policy, so it should start none.
///
/// On Windows it takes `THREAD_PRIORITY_TIME_CRITICAL`, the top of the
/// normal range; anything higher needs the realtime class and admin rights.
#[cfg(target_os = "linux")]
pub fn raise_thread_priority() -> io::Result<()> {
    const SCHED_FIFO: i32 = 1;
    const EPERM: i32 = 1;
    #[repr(C)]
    struct SchedParam {
        priority: i32,
    }
    unsafe extern "C" {
        fn pthread_self() -> c_ulong;
        fn pthread_setschedparam(thread: c_ulong, policy: i32, param: *const SchedParam) -> i32;
    }
    let param = SchedParam {
        priority: REALTIME_PRIORITY,
    };
    match unsafe { pthread_setschedparam(pthread_self(), SCHED_FIFO, &param) } {
        0 => Ok(()),
        EPERM => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "SCHED_FIFO {REALTIME_PRIORITY} refused; allow it with LimitRTPRIO={REALTIME_PRIORITY} in the systemd unit"
            ),
        )),
        code => Err(io::Error::from_raw_os_error(code)),
    }
}

#[cfg(windows)]
pub fn raise_thread_priority() -> io::Result<()> {
    const THREAD_PRIORITY_TIME_CRITICAL: i32 = 15;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThread() -> *mut c_void;
        fn SetThreadPriority(thread: *mut c_void, priority: i32) -> i32;
    }
    if unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn raise_thread_priority() -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}

/// Resolves on Ctrl+C, or on SIGTERM where it exists: the signal service
/// managers (systemd, Docker) stop a process with. Returns which one came.
pub async fn shutdown_signal() -> &'static str {
    #[cfg(unix)]
    {
        let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
        tokio::select! {
            Ok(()) = tokio::signal::ctrl_c() => "Ctrl+C",
            _ = term.recv() => "SIGTERM",
        }
    }
    #[cfg(not(unix))]
    {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
        "Ctrl+C"
    }
}
