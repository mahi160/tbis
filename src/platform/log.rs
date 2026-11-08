#[cfg(any(test, target_os = "windows"))]
const SENSITIVE: [&str; 25] = [
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
    "username",
    "userinfo",
    "signature",
    "sig=",
    "key=",
    "://",
    "file:",
    "data:",
];

/// Points the process's stderr at `tbis.log` next to the exe. Rust's stderr looks up
/// the std handle on every write, so every `eprintln!` lands in the file. If the file
/// can't be created (read-only install folder) stderr is left alone.
#[cfg(target_os = "windows")]
pub fn init() {
    std::panic::set_hook(Box::new(|info| {
        let message = info
            .payload()
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| info.payload().downcast_ref::<&str>().copied())
            .unwrap_or("non-string panic");
        let lower = message.to_ascii_lowercase();
        let message = if SENSITIVE.iter().any(|marker| lower.contains(marker)) {
            "[private panic text omitted]"
        } else {
            message
        };
        let location = info.location().map(ToString::to_string).unwrap_or_default();
        eprintln!("panic at {location}: {message}");
    }));
    let Ok(path) = std::env::current_exe().map(|exe| exe.with_file_name("tbis.log")) else {
        return;
    };
    let Ok(file) = std::fs::File::create(path) else {
        return;
    };
    use std::os::windows::io::IntoRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::{STD_ERROR_HANDLE, SetStdHandle};
    let handle = file.into_raw_handle();
    // The handle stays open for the life of the process.
    let _ = unsafe { SetStdHandle(STD_ERROR_HANDLE, HANDLE(handle)) };
    eprintln!("tbis {} Windows runtime", env!("CARGO_PKG_VERSION"));
}

#[cfg(any(test, target_os = "windows"))]
pub fn redact(message: &str, secrets: &[String]) -> String {
    let mut message = message.to_owned();
    for secret in secrets {
        message = message.replace(secret, "[redacted]");
    }
    message
        .lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            match SENSITIVE
                .iter()
                .filter_map(|marker| lower.find(marker))
                .min()
            {
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
    eprintln!("{message}");
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn redacts_secrets_and_markers() {
        let secrets = vec!["abc123".to_owned()];
        assert_eq!(
            redact("opening abc123 now", &secrets),
            "opening [redacted] now"
        );
        assert_eq!(
            redact("GET https://host/Videos?api_key=x", &[]),
            "GET [redacted]"
        );
        assert_eq!(
            redact("Authorization: MediaBrowser Token=z", &[]),
            "[redacted]"
        );
    }

    #[test]
    fn keeps_ordinary_lines() {
        let line = r"cache: C:\Users\me\clip.mkv (seekable?) a@b header ok session 1";
        assert_eq!(redact(line, &[]), line);
        assert_eq!(redact("/tmp/a.mkv: unknown?", &[]), "/tmp/a.mkv: unknown?");
    }
}
