// SPDX-License-Identifier: MPL-2.0

//! Serialize the complete VT bounce across users and direct helper invocations.
//! Never unlink the lock: removing it would let callers lock different inodes.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub(crate) fn acquire() -> io::Result<File> {
    if !crate::is_root() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "root required",
        ));
    }
    // /run is root-controlled. Do not accept a caller-selected path or use /tmp.
    acquire_at(Path::new("/run/cosmic-nightlight-helper.lock"), 0)
}

fn acquire_at(path: &Path, owner: u32) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != owner
        || metadata.nlink() != 1
        || metadata.mode() & 0o077 != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unsafe helper lock file",
        ));
    }
    // Fail promptly instead of allowing unbounded queues of privileged helpers.
    // SAFETY: file owns a live fd. flock does not retain pointers.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn excludes_concurrent_callers_and_rejects_unsafe_files() {
        let dir = std::env::temp_dir().join(format!("nightlight-lock-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("lock");
        // SAFETY: geteuid has no preconditions.
        let uid = unsafe { libc::geteuid() };
        let first = acquire_at(&path, uid).unwrap();
        assert_eq!(
            acquire_at(&path, uid).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        drop(first);
        drop(acquire_at(&path, uid).unwrap());
        assert!(acquire_at(&path, uid.wrapping_add(1)).is_err());
        let link = dir.join("symlink");
        symlink(&path, &link).unwrap();
        assert!(acquire_at(&link, uid).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();
        assert!(acquire_at(&path, uid).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::hard_link(&path, dir.join("hardlink")).unwrap();
        assert!(acquire_at(&path, uid).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
