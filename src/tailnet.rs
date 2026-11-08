//! Serialized node lifecycle and a loopback-only TCP bridge (ADR-0007).

use std::collections::HashMap;
use std::io;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::tailscale::{BackendState, Node, Status, delete_state};
use anyhow::{Context as _, Result};

#[derive(Clone)]
pub struct Service(Arc<Inner>);

struct Inner {
    dir: PathBuf,
    runtime: Mutex<Runtime>,
}

#[derive(Default)]
struct Runtime {
    node: Option<Arc<Node>>,
    dialer: Arc<Mutex<Option<Arc<Node>>>>,
    bridge: Option<(String, Bridge)>,
}

impl Service {
    pub fn new() -> Self {
        Self(Arc::new(Inner {
            dir: crate::support_dir::app_support_dir().join("tailscale"),
            runtime: Mutex::new(Runtime::default()),
        }))
    }

    /// Blocking operations are called only on the background executor.
    pub fn start_status(&self) -> Result<Status> {
        let mut runtime = self.0.runtime.lock().unwrap();
        if runtime.node.is_none() {
            let mut name = [0u8; 256];
            // SAFETY: writable buffer of the supplied size; hostname is bounded below.
            if unsafe { libc::gethostname(name.as_mut_ptr().cast(), name.len()) } != 0 {
                return Err(io::Error::last_os_error()).context("reading this computer's hostname");
            }
            let end = name.iter().position(|b| *b == 0).unwrap_or(name.len());
            let host = String::from_utf8_lossy(&name[..end]);
            let host = host.split('.').next().unwrap_or("computer");
            let node = Arc::new(Node::start(&self.0.dir, &format!("tbis-{host}"))?);
            *runtime.dialer.lock().unwrap() = Some(node.clone());
            runtime.node = Some(node);
        }
        runtime
            .node
            .as_ref()
            .context("Tailscale did not start")?
            .status()
    }

    pub fn status_label(&self) -> Result<&'static str> {
        let runtime = self.0.runtime.lock().unwrap();
        match &runtime.node {
            Some(node) if node.status()?.backend_state == BackendState::Running => Ok("connected"),
            Some(_) => Ok("needs login"),
            None => Ok("off"),
        }
    }

    pub fn forward(&self, address: &str) -> Result<String> {
        let mut runtime = self.0.runtime.lock().unwrap();
        if let Some((destination, bridge)) = &runtime.bridge
            && destination == address
        {
            return Ok(bridge.server.clone());
        }
        let dialer = runtime.dialer.clone();
        let destination = address.to_owned();
        let bridge = Bridge::start(move || {
            let node = dialer
                .lock()
                .unwrap()
                .clone()
                .context("Tailscale needs authorization")?;
            node.dial(&destination)
        })?;
        let server = bridge.server.clone();
        runtime.bridge = Some((address.to_owned(), bridge));
        Ok(server)
    }

    pub fn log_out(&self) -> Result<()> {
        // Hold the lifecycle lock through deletion so no new start can reuse old state.
        let mut runtime = self.0.runtime.lock().unwrap();
        runtime.close_node()?;
        if let Some((_, bridge)) = &runtime.bridge {
            bridge.disconnect()?;
        }
        delete_state(&self.0.dir)
    }
}

impl Runtime {
    fn close_node(&mut self) -> Result<()> {
        *self.dialer.lock().unwrap() = None;
        self.node.take().map_or(Ok(()), |node| node.close())
    }

    fn stop(&mut self) -> Result<()> {
        if let Some((_, bridge)) = &self.bridge {
            bridge.stopped.store(true, Ordering::SeqCst);
        }
        let result = self.close_node();
        self.bridge.take();
        result
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        let mut runtime = std::mem::take(self.runtime.get_mut().unwrap());
        // tsnet shutdown can flush logs; never wait for it on the UI thread.
        thread::spawn(move || {
            if let Err(err) = runtime.stop() {
                eprintln!("closing Tailscale failed: {err}");
            }
        });
    }
}

struct Bridge {
    server: String,
    stopped: Arc<AtomicBool>,
    listener: Option<JoinHandle<()>>,
    disconnect: mpsc::Sender<mpsc::Sender<()>>,
}

struct Connection {
    local: TcpStream,
    remote: Arc<Mutex<Option<UnixStream>>>,
    worker: JoinHandle<()>,
}

fn close_connections(connections: &mut HashMap<u64, Connection>) {
    for connection in connections.values() {
        let _ = connection.local.shutdown(Shutdown::Both);
        if let Some(remote) = &*connection.remote.lock().unwrap() {
            let _ = remote.shutdown(Shutdown::Both);
        }
    }
    for (_, connection) in connections.drain() {
        let _ = connection.worker.join();
    }
}

impl Bridge {
    fn disconnect(&self) -> Result<()> {
        let (done, received) = mpsc::channel();
        self.disconnect
            .send(done)
            .context("stopping tailnet connections")?;
        received
            .recv()
            .context("waiting for tailnet connections to stop")
    }

    fn start(dial: impl Fn() -> Result<UnixStream> + Send + Sync + 'static) -> Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let server = format!("http://{}", listener.local_addr()?);
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let dial = Arc::new(dial);
        let (disconnect, requests) = mpsc::channel::<mpsc::Sender<()>>();
        let worker = thread::Builder::new()
            .name("tailnet-bridge".into())
            .spawn(move || {
                let mut connections: HashMap<u64, Connection> = HashMap::new();
                let mut next_id = 0;
                while !stop.load(Ordering::SeqCst) {
                    while let Ok(done) = requests.try_recv() {
                        close_connections(&mut connections);
                        let _ = done.send(());
                    }
                    connections.retain(|_, connection| !connection.worker.is_finished());
                    match listener.accept() {
                        Ok((mut local, _)) => {
                            // Darwin accepted sockets inherit the listener's nonblocking flag.
                            if local.set_nonblocking(false).is_err() {
                                continue;
                            }
                            let Ok(control) = local.try_clone() else {
                                continue;
                            };
                            let remote = Arc::new(Mutex::new(None));
                            let remote_control = remote.clone();
                            let dial = dial.clone();
                            let stop = stop.clone();
                            let worker = thread::spawn(move || {
                                let result = (|| -> Result<()> {
                                    let mut upstream = dial()?;
                                    let mut state = remote.lock().unwrap();
                                    if stop.load(Ordering::SeqCst) {
                                        return Ok(());
                                    }
                                    *state = Some(upstream.try_clone()?);
                                    drop(state);
                                    let mut upload = upstream.try_clone()?;
                                    let mut source = local.try_clone()?;
                                    thread::scope(|scope| {
                                        scope.spawn(move || {
                                            if io::copy(&mut source, &mut upload).is_err() {
                                                let _ = upload.shutdown(Shutdown::Both);
                                                let _ = source.shutdown(Shutdown::Both);
                                            } else {
                                                let _ = upload.shutdown(Shutdown::Write);
                                            }
                                        });
                                        if io::copy(&mut upstream, &mut local).is_err() {
                                            let _ = upstream.shutdown(Shutdown::Both);
                                            let _ = local.shutdown(Shutdown::Both);
                                        } else {
                                            let _ = local.shutdown(Shutdown::Write);
                                        }
                                    });
                                    Ok(())
                                })();
                                if result.is_err() {
                                    let _ = local.shutdown(Shutdown::Both);
                                }
                            });
                            connections.insert(
                                next_id,
                                Connection {
                                    local: control,
                                    remote: remote_control,
                                    worker,
                                },
                            );
                            next_id += 1;
                        }
                        Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(20))
                        }
                        Err(_) => break,
                    }
                }
                close_connections(&mut connections);
            })?;
        Ok(Self {
            server,
            stopped,
            listener: Some(worker),
            disconnect,
        })
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        if let Some(listener) = self.listener.take() {
            let _ = listener.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use std::io::{Read as _, Write as _};

    #[test]
    fn bridge_is_loopback_only_full_duplex_and_closes_on_drop() {
        let (upstream, mut server) = UnixStream::pair().unwrap();
        let upstream = Mutex::new(Some(upstream));
        let bridge = Bridge::start(move || {
            upstream
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| anyhow!("one connection"))
        })
        .unwrap();
        let address = bridge.server.trim_start_matches("http://");
        assert!(address.starts_with("127.0.0.1:"));
        let mut client = TcpStream::connect(address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        client.write_all(b"request").unwrap();
        let mut request = [0; 7];
        server.read_exact(&mut request).unwrap();
        assert_eq!(&request, b"request");
        server.write_all(b"response").unwrap();
        let mut response = [0; 8];
        client.read_exact(&mut response).unwrap();
        assert_eq!(&response, b"response");
        let address = address.to_owned();
        drop(bridge);
        assert_eq!(client.read(&mut [0]).unwrap(), 0);
        assert!(TcpStream::connect(address).is_err());
    }

    #[test]
    fn disconnect_keeps_the_route_available_for_reconnection() {
        let (send, receive) = mpsc::channel();
        let bridge = Bridge::start(move || {
            let (upstream, server) = UnixStream::pair()?;
            send.send(server)?;
            Ok(upstream)
        })
        .unwrap();
        let address = bridge.server.trim_start_matches("http://");
        for _ in 0..2 {
            let mut client = TcpStream::connect(address).unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut server = receive.recv_timeout(Duration::from_secs(2)).unwrap();
            server.write_all(b"connected").unwrap();
            let mut message = [0; 9];
            client.read_exact(&mut message).unwrap();
            assert_eq!(&message, b"connected");
            bridge.disconnect().unwrap();
            assert_eq!(client.read(&mut [0]).unwrap(), 0);
        }
    }

    #[test]
    fn logout_without_started_node_is_repeatable_and_removes_state() {
        let dir = std::env::temp_dir().join(format!("tbis-tailnet-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("state"), b"test").unwrap();
        let service = Service(Arc::new(Inner {
            dir: dir.clone(),
            runtime: Mutex::new(Runtime::default()),
        }));
        service.log_out().unwrap();
        service.log_out().unwrap();
        assert!(!dir.exists());
    }
}
