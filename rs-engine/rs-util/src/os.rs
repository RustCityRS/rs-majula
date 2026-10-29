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
