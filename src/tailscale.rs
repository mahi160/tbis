//! In-process tailnet node: a safe wrapper over libtailscale's C API
//! (`vendor/libtailscale/tailscale.h`), linked by `build.rs` (ADR-0007).

use std::ffi::{CStr, CString, c_char, c_int};
use std::net::Ipv4Addr;
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

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

pub fn parse_address(input: &str) -> Result<String> {
    let (host, port) = match input.split_once(':') {
        Some((host, port)) => {
            let port: u16 = port.parse().context("invalid tailnet port")?;
            if port == 0 {
                return Err(anyhow!("invalid tailnet port"));
            }
            (host, port)
        }
        None => (input, 8096),
    };
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        let octets = ip.octets();
        if octets[0] != 100 || !(64..=127).contains(&octets[1]) {
            return Err(anyhow!("IP address is not in the tailnet range"));
        }
    } else {
        let valid = !host.is_empty()
            && host.len() <= 253
            && host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && label.as_bytes()[0].is_ascii_alphanumeric()
                    && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                    && label
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            })
            && host.bytes().any(|c| c.is_ascii_alphabetic());
        if !valid {
            return Err(anyhow!("invalid tailnet hostname"));
        }
    }
    Ok(format!("{host}:{port}"))
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
    #[cfg(test)]
    dir: PathBuf,
    closed: AtomicBool,
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
            #[cfg(test)]
            dir: dir.to_path_buf(),
            closed: AtomicBool::new(false),
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

    pub(crate) fn close(&self) -> Result<()> {
        if !self.closed.swap(true, Ordering::SeqCst) {
            // SAFETY: close removes this handle once; libtailscale synchronizes handle lookup.
            self.check(unsafe { tailscale_close(self.sd) })?;
        }
        Ok(())
    }

    /// Closes the node and deletes its state, so the next start needs a fresh login.
    #[cfg(test)]
    pub fn log_out(self) -> Result<()> {
        self.close()?;
        delete_state(&self.dir)
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
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).context("deleting the Tailscale state dir"),
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            crate::log::write(&format!("closing Tailscale failed: {err}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendState, Node, parse_address};
    use std::time::{Duration, Instant};

    #[test]
    fn tailnet_address_validation() {
        assert_eq!(parse_address("jellyfin").unwrap(), "jellyfin:8096");
        assert_eq!(
            parse_address("host.tail123.ts.net:1234").unwrap(),
            "host.tail123.ts.net:1234"
        );
        assert_eq!(parse_address("100.64.0.1").unwrap(), "100.64.0.1:8096");
        assert_eq!(
            parse_address("100.127.255.255:80").unwrap(),
            "100.127.255.255:80"
        );
        for input in [
            "",
            "http://host",
            "host/path",
            "host:0",
            "host:65536",
            "100.63.0.1",
            "100.128.0.1",
            "192.168.1.1",
            "100.64.0.999",
            "-host",
            "host..name",
            "[::1]:80",
        ] {
            assert!(parse_address(input).is_err(), "{input}");
        }
    }

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
