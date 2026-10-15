// SPDX-License-Identifier: MPL-2.0

//! Virtual-terminal juggling.
//!
//! cosmic-comp holds the DRM master lock while it is the foreground session,
//! so another process cannot set the CRTC gamma. Switching to a spare VT
//! makes logind revoke the compositor's DRM master; while we sit on that VT
//! a privileged process can grab master, write the gamma LUTs, and the
//! values survive the switch back because the compositor does not reset
//! them. This module performs that switch-and-restore dance.

use std::fs::OpenOptions;
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

// VT ioctls from <linux/vt.h>.
const VT_OPENQRY: libc::c_ulong = 0x5600;
const VT_GETSTATE: libc::c_ulong = 0x5603;
const VT_ACTIVATE: libc::c_ulong = 0x5606;
const SWITCH_TIMEOUT: Duration = Duration::from_secs(3);

#[repr(C)]
struct VtStat {
    v_active: libc::c_ushort,
    v_signal: libc::c_ushort,
    v_state: libc::c_ushort,
}

fn errno() -> io::Error {
    io::Error::last_os_error()
}

fn current_vt(fd: RawFd) -> io::Result<libc::c_int> {
    let mut stat = VtStat {
        v_active: 0,
        v_signal: 0,
        v_state: 0,
    };
    // SAFETY: `fd` is a live console fd; `stat` is a valid VtStat.
    let rc = unsafe { libc::ioctl(fd, VT_GETSTATE, &mut stat as *mut VtStat) };
    if rc < 0 {
        return Err(errno());
    }
    Ok(stat.v_active as libc::c_int)
}

fn query_free_vt(fd: RawFd) -> io::Result<libc::c_int> {
    let mut vt: libc::c_int = 0;
    // SAFETY: `fd` is a live console fd; VT_OPENQRY writes an int.
    let rc = unsafe { libc::ioctl(fd, VT_OPENQRY, &mut vt as *mut libc::c_int) };
    if rc < 0 {
        return Err(errno());
    }
    Ok(vt)
}

fn activate(fd: RawFd, vt: libc::c_int) -> io::Result<()> {
    // SAFETY: `fd` is a live console fd; VT_ACTIVATE takes an int by value.
    let rc = unsafe { libc::ioctl(fd, VT_ACTIVATE, vt) };
    if rc < 0 {
        return Err(errno());
    }
    wait_for_vt(vt, SWITCH_TIMEOUT, || current_vt(fd))
}

/// Restores the original VT when dropped, so we return the user to their
/// session even if the closure returns early or panics.
struct VtRestore {
    fd: RawFd,
    original: libc::c_int,
}

impl Drop for VtRestore {
    fn drop(&mut self) {
        let _ = activate(self.fd, self.original);
    }
}

/// Switches to a spare VT (making the compositor drop DRM master), runs
/// `f` while master is available, then switches back to the original VT.
///
/// `session_vt` is the caller's known graphical-session VT (from `XDG_VTNR`):
/// - `Some(n)`: we **guard** against bouncing at the wrong moment — if the
///   foreground VT isn't `n` (e.g. we're mid login/greeter handoff), we refuse
///   to switch and return an error rather than risk parking the seat on the
///   spare TTY. We also restore to `n` directly, never to a stale snapshot.
/// - `None`: manual/CLI use — fall back to snapshotting whatever VT is
///   foreground and restoring to it, with no guard.
///
/// Requires `CAP_SYS_TTY_CONFIG` (i.e. root) to open `/dev/tty0` and issue
/// the VT ioctls.
pub fn with_master_window<T>(
    session_vt: Option<libc::c_int>,
    f: impl FnOnce() -> T,
) -> io::Result<T> {
    let tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty0")?;
    let fd = tty.as_raw_fd();

    let active = current_vt(fd)?;
    let original = match session_vt {
        Some(vt) => {
            // Only bounce when the graphical session is actually foreground;
            // otherwise switching VTs could strand the user on the spare TTY.
            if active != vt {
                return Err(io::Error::other(format!(
                    "active VT is {active}, not the session VT {vt}; skipping VT bounce"
                )));
            }
            vt
        }
        None => active,
    };
    let spare = query_free_vt(fd)?;

    // Arm restoration before switching: even a timed-out activation may
    // already have left the original VT.
    let _restore = VtRestore { fd, original };
    activate(fd, spare)?;

    // Give logind a moment to revoke the compositor's DRM master after the
    // switch completes before we try to grab it.
    std::thread::sleep(Duration::from_millis(150));

    Ok(f())
}

fn wait_for_vt(
    vt: libc::c_int,
    timeout: Duration,
    mut current: impl FnMut() -> io::Result<libc::c_int>,
) -> io::Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        if current()? == vt {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "virtual terminal switch timed out",
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_wait_is_bounded_and_reports_query_failures() {
        assert!(wait_for_vt(2, Duration::ZERO, || Ok(2)).is_ok());
        assert_eq!(
            wait_for_vt(2, Duration::ZERO, || Ok(1)).unwrap_err().kind(),
            io::ErrorKind::TimedOut,
        );
        assert_eq!(
            wait_for_vt(2, SWITCH_TIMEOUT, || Err(
                io::ErrorKind::PermissionDenied.into()
            ))
            .unwrap_err()
            .kind(),
            io::ErrorKind::PermissionDenied,
        );
    }
}
