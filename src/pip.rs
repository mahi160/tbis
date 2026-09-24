//! PiP (ADR-0003): hands playback to a spawned standalone `mpv` window, talks to it over JSON IPC.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use futures::channel::mpsc::UnboundedSender;
use serde_json::{Value, json};

const POLL_EVERY: Duration = Duration::from_millis(500);

pub enum PipEvent {
    Position(f64),
    /// PiP window closed; last position it reached.
    Ended(f64),
}

pub struct PipStart<'a> {
    pub url: &'a str,
    pub auth_header: &'a str,
    pub start_seconds: f64,
    pub volume: f64,
    pub muted: bool,
    pub audio_track: Option<i64>,
    pub subtitle_track: Option<i64>,
}

/// Running PiP window; dropping it closes the window.
pub struct Pip {
    child: Child,
}

impl Pip {
    pub fn start(args: PipStart, events: UnboundedSender<PipEvent>) -> Result<Self, String> {
        let socket = std::env::temp_dir().join(format!("tbis-pip-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket); // stale from crash
        let track = |id: Option<i64>| id.map_or("no".to_string(), |id| id.to_string());
        // no URL or token in argv: world-readable via ps; sent over IPC instead
        let child = Command::new(mpv_binary())
            .arg("--idle=once") // exits after the file ends
            .arg("--no-border")
            .arg("--ontop")
            .arg("--on-all-workspaces")
            .arg("--autofit=640x360")
            .arg("--geometry=-24-24") // bottom-right corner
            .arg("--ytdl=no")
            .arg(format!("--input-ipc-server={}", socket.display()))
            .arg(format!("--start={}", args.start_seconds))
            .arg(format!("--volume={}", args.volume.round()))
            .arg(format!("--mute={}", if args.muted { "yes" } else { "no" }))
            .arg(format!("--aid={}", track(args.audio_track)))
            .arg(format!("--sid={}", track(args.subtitle_track)))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Cannot start PiP: {e}"))?;

        let setup = [
            json!([
                "change-list",
                "http-header-fields",
                "append",
                format!("Authorization: {}", args.auth_header)
            ]),
            json!(["loadfile", args.url, "replace"]),
        ];
        let start = args.start_seconds;
        std::thread::spawn(move || {
            let last = poll(&socket, &setup, start, &events);
            let _ = std::fs::remove_file(&socket);
            let _ = events.unbounded_send(PipEvent::Ended(last));
        });
        Ok(Self { child })
    }

    /// Closes the window; `Ended` follows.
    pub fn stop(&mut self) {
        let _ = self.child.kill();
    }
}

impl Drop for Pip {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait(); // reap zombie
    }
}

/// Sends setup commands, then polls time-pos until mpv exits. Returns last position.
fn poll(socket: &PathBuf, setup: &[Value], start: f64, events: &UnboundedSender<PipEvent>) -> f64 {
    // mpv creates socket shortly after spawn
    let mut stream = None;
    for _ in 0..50 {
        if let Ok(s) = UnixStream::connect(socket) {
            stream = Some(s);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let Some(stream) = stream else {
        return start;
    };
    let Ok(read_half) = stream.try_clone() else {
        return start;
    };
    let mut ipc = Ipc {
        writer: stream,
        reader: BufReader::new(read_half),
        next_id: 0,
    };
    for command in setup {
        if ipc.request(command).is_none() {
            return start;
        }
    }
    let mut last = start;
    loop {
        match ipc.request(&json!(["get_property", "time-pos"])) {
            None => return last,
            Some(reply) => {
                // no time-pos while loading
                if let Some(position) = reply.get("data").and_then(Value::as_f64) {
                    last = position;
                    if events.unbounded_send(PipEvent::Position(position)).is_err() {
                        return last;
                    }
                }
            }
        }
        std::thread::sleep(POLL_EVERY);
    }
}

struct Ipc {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
    next_id: u64,
}

impl Ipc {
    /// One command and its reply, skipping unprompted event lines. `None` once mpv is gone.
    fn request(&mut self, command: &Value) -> Option<Value> {
        self.next_id += 1;
        let line = json!({ "command": command, "request_id": self.next_id }).to_string() + "\n";
        self.writer.write_all(line.as_bytes()).ok()?;
        loop {
            let mut reply = String::new();
            if self.reader.read_line(&mut reply).ok()? == 0 {
                return None;
            }
            let Ok(reply) = serde_json::from_str::<Value>(&reply) else {
                continue;
            };
            if reply.get("request_id").and_then(Value::as_u64) == Some(self.next_id) {
                return Some(reply);
            }
        }
    }
}

/// Finder/Dock launches lack the shell PATH, so try Homebrew paths first.
fn mpv_binary() -> PathBuf {
    ["/opt/homebrew/bin/mpv", "/usr/local/bin/mpv"]
        .into_iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("mpv"))
}
