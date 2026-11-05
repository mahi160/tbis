#[cfg(target_os = "windows")]
use std::fs::File;
#[cfg(target_os = "windows")]
use std::io::Write;
#[cfg(target_os = "windows")]
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "windows")]
static FILE: OnceLock<Mutex<File>> = OnceLock::new();

#[cfg(target_os = "windows")]
pub fn init() -> std::io::Result<()> {
    if FILE.get().is_some() {
        return Ok(());
    }
    let path = std::env::current_exe()?.with_file_name("tbis.log");
    let file = File::create(path)?;
    let _ = FILE.set(Mutex::new(file));
    std::panic::set_hook(Box::new(|info| {
        let message = info
            .payload()
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| info.payload().downcast_ref::<&str>().copied())
            .unwrap_or("non-string panic");
        let lower = message.to_ascii_lowercase();
        let private = [
            "://",
            "authorization",
            "bearer",
            "basic ",
            "token",
            "password",
            "secret",
            "cookie",
            "credential",
            "api_key",
            "api-key",
            "x-emby",
        ]
        .iter()
        .any(|marker| lower.contains(marker));
        let message = if private {
            "[private panic text omitted]"
        } else {
            message
        };
        let location = info.location().map(ToString::to_string).unwrap_or_default();
        write(&format!("panic at {location}: {message}"));
    }));
    write(concat!(
        "tbis ",
        env!("CARGO_PKG_VERSION"),
        " Windows runtime; GPU behavior unverified"
    ));
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn redact(message: &str, secrets: &[String], redact_paths: bool) -> String {
    let mut message = message.to_owned();
    for secret in secrets {
        message = message.replace(secret, "[redacted]");
    }
    message
        .lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            let sensitive = [
                "authorization",
                "authenticate",
                "bearer",
                "basic ",
                "token",
                "password",
                "passwd",
                "pass=",
                "user=",
                "secret",
                "cookie",
                "credential",
                "api_key",
                "api-key",
                "apikey",
                "x-emby",
                "x-mediabrowser",
                "header",
                "username",
                "userinfo",
                "session",
                "signature",
                "sig=",
                "key=",
                "://",
                "file:",
                "data:",
                "?",
                "@",
            ]
            .iter()
            .filter_map(|marker| lower.find(marker))
            .min();
            let path = redact_paths.then(|| line.find(['/', '\\'])).flatten();
            match sensitive.into_iter().chain(path).min() {
                Some(end) => {
                    let start = line[..end]
                        .char_indices()
                        .rev()
                        .find(|(_, ch)| ch.is_whitespace() || matches!(ch, '\'' | '"' | '(' | '['))
                        .map_or(0, |(index, ch)| index + ch.len_utf8());
                    format!("{}[redacted]", &line[..start])
                }
                None => line.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn write(message: &str) {
    #[cfg(target_os = "windows")]
    if let Some(file) = FILE.get() {
        let mut file = file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = writeln!(file, "{message}");
        let _ = file.flush();
        return;
    }
    eprintln!("{message}");
}
