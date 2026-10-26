//! In-process tailnet node: a safe wrapper over libtailscale's C API
//! (`vendor/libtailscale/tailscale.h`), linked by `build.rs` (ADR-0007).
// nothing calls this until sign-in over the tailnet lands
#![allow(dead_code)]

use std::ffi::{CStr, CString, c_char, c_int};
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow};
use serde::Deserialize;

unsafe extern "C" {
    fn tailscale_new() -> c_int;
    fn tailscale_start(sd: c_int) -> c_int;
    fn tailscale_close(sd: c_int) -> c_int;
    fn tailscale_set_dir(sd: c_int, dir: *const c_char) -> c_int;
    fn tailscale_set_hostname(sd: c_int, hostname: *const c_char) -> c_int;
    fn tailscale_set_logfd(sd: c_int, fd: c_int) -> c_int;
    fn tailscale_dial(
        sd: c_int,
        network: *const c_char,
        addr: *const c_char,
        conn_out: *mut c_int,
    ) -> c_int;
    fn tailscale_status_json(sd: c_int, json_out: *mut *mut c_char) -> c_int;
    fn tailscale_errmsg(sd: c_int, buf: *mut c_char, buflen: usize) -> c_int;
}

/// Where the node is in its login lifecycle (ipnstate `BackendState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum BackendState {
    NeedsLogin,
    NeedsMachineAuth,
    Starting,
    Running,
    Stopped,
    /// `NoState` and anything newer.
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Status {
    pub backend_state: BackendState,
    /// Login page to show while `NeedsLogin`.
    #[serde(rename = "AuthURL", default)]
    pub auth_url: Option<String>,
}

/// A started tailnet node; closed on drop. Closing waits for the node to shut down,
/// so drop it off the UI thread.
pub struct Node {
    sd: c_int,
    dir: PathBuf,
}

impl Node {
    /// Starts a node keeping its state in `dir` and joining as `hostname`.
    /// Returns at once: login and connection progress show in `status`.
    pub fn start(dir: &Path, hostname: &str) -> Result<Self> {
        std::fs::create_dir_all(dir).context("creating the Tailscale state dir")?;
        let dir_c = CString::new(dir.to_str().context("non-UTF-8 state dir")?)?;
        let hostname_c = CString::new(hostname)?;
        // SAFETY: no arguments; returns a handle we own from here on.
        let node = Self {
            sd: unsafe { tailscale_new() },
            dir: dir.to_path_buf(),
        };
        // SAFETY: valid handle and NUL-terminated strings that outlive each call.
        unsafe {
            // tsnet logs a lot to stderr by default
            node.check(tailscale_set_logfd(node.sd, -1))?;
            node.check(tailscale_set_dir(node.sd, dir_c.as_ptr()))?;
            node.check(tailscale_set_hostname(node.sd, hostname_c.as_ptr()))?;
            node.check(tailscale_start(node.sd))?;
        }
        Ok(node)
    }

    pub fn status(&self) -> Result<Status> {
        let mut json: *mut c_char = std::ptr::null_mut();
        // SAFETY: valid handle; on success `json` is a malloc'd C string we must free.
        self.check(unsafe { tailscale_status_json(self.sd, &mut json) })?;
        // SAFETY: NUL-terminated on success; freed once, after its last use.
        let parsed = serde_json::from_slice::<Status>(unsafe { CStr::from_ptr(json) }.to_bytes());
        unsafe { libc::free(json.cast()) };
        let mut status = parsed?;
        status.auth_url = status.auth_url.filter(|url| !url.is_empty());
        Ok(status)
    }

    /// Opens a TCP connection to `addr` (`host:port`; MagicDNS names work) over the tailnet.
    /// Blocks with no timeout until connected, so call it off the UI thread.
    /// Streams outlive `Node`: close them before `log_out`.
    pub fn dial(&self, addr: &str) -> Result<UnixStream> {
        let addr = CString::new(addr)?;
        let mut fd: c_int = -1;
        // SAFETY: valid handle and strings; on success `fd` is a socket we now own.
        self.check(unsafe { tailscale_dial(self.sd, c"tcp".as_ptr(), addr.as_ptr(), &mut fd) })?;
        // SAFETY: a fresh socketpair end that nothing else closes.
        Ok(unsafe { UnixStream::from_raw_fd(fd) })
    }

    /// Closes the node and deletes its state, so the next start needs a fresh login.
    pub fn log_out(self) -> Result<()> {
        let dir = self.dir.clone();
        drop(self);
        delete_state(&dir)
    }

    fn check(&self, code: c_int) -> Result<()> {
        if code == 0 {
            return Ok(());
        }
        let mut buf = [0 as c_char; 256];
        // SAFETY: the buffer's length is passed; it's NUL-terminated after, even when
        // truncated (ERANGE) or for a bad handle (EBADF, empty).
        unsafe { tailscale_errmsg(self.sd, buf.as_mut_ptr(), buf.len()) };
        let message = unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy();
        if message.is_empty() {
            return Err(anyhow!("Tailscale error {code}"));
        }
        Err(anyhow!("Tailscale: {message}"))
    }
}

/// Deletes a node's state from `dir`, signing that device out; for when no `Node` is running.
pub fn delete_state(dir: &Path) -> Result<()> {
    std::fs::remove_dir_all(dir).context("deleting the Tailscale state dir")
}

impl Drop for Node {
    fn drop(&mut self) {
        // SAFETY: we own the handle and never use it after this.
        unsafe { tailscale_close(self.sd) };
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendState, Node};
    use std::time::{Duration, Instant};

    #[test]
    #[ignore = "needs network: reaches Tailscale's control server"]
    fn fresh_node_needs_login_with_auth_url() {
        let dir = std::env::temp_dir().join(format!("tbis-tailscale-{}", uuid::Uuid::new_v4()));
        let node = Node::start(&dir, "tbis-test").unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            let status = node.status().unwrap();
            if status.backend_state == BackendState::NeedsLogin && status.auth_url.is_some() {
                break status;
            }
            assert!(Instant::now() < deadline, "no auth URL: {status:?}");
            std::thread::sleep(Duration::from_millis(250));
        };
        assert!(status.auth_url.unwrap().starts_with("https://"));
        node.log_out().unwrap();
        assert!(!dir.exists(), "log out deletes the state");
    }
}
