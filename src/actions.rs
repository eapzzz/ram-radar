use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::{fs, io};
/// Open a stable process handle before checking identity, then signal that handle.
pub fn terminate(pid: u32, expected_start: u64) -> io::Result<()> {
    if pid <= 1 || pid == std::process::id() {
        return Err(io::Error::other("This process is protected"));
    }
    let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    let fd = unsafe { OwnedFd::from_raw_fd(raw as i32) };
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    if starttime(&stat) != Some(expected_start) {
        return Err(io::Error::other(
            "Process identity changed; refresh and try again",
        ));
    }
    let result = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd.as_raw_fd(),
            libc::SIGTERM,
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn starttime(stat: &str) -> Option<u64> {
    stat.get(stat.rfind(')')? + 1..)?
        .split_whitespace()
        .nth(19)?
        .parse()
        .ok()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn never_terminate_self_or_init() {
        assert!(terminate(1, 0).is_err());
        assert!(terminate(std::process::id(), 0).is_err());
    }
    #[test]
    fn mismatched_identity_does_not_signal() {
        let mut child = std::process::Command::new("sleep")
            .arg("10")
            .spawn()
            .unwrap();
        assert!(terminate(child.id(), u64::MAX).is_err());
        assert!(child.try_wait().unwrap().is_none());
        child.kill().unwrap();
        child.wait().unwrap();
    }
}
