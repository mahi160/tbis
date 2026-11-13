//! The app's one logging path: every diagnostic line goes through `write`, and text that
//! may carry credentials or private paths is scrubbed with `redact` first. Where the lines
//! end up is the platform's business (stderr; a file on Windows, see `platform::init`).

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

/// Whether `text` mentions anything that may be a credential or private location.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))] // only Windows' panic hook asks
pub fn is_private(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    SENSITIVE.iter().any(|marker| lower.contains(marker))
}

/// The header itself plus each quoted value in it (e.g. the token), to scrub from logs.
pub fn header_secrets(header: &str) -> Vec<String> {
    std::iter::once(header)
        .chain(header.split('"').skip(1).step_by(2))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

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
    use super::{header_secrets, redact};

    #[test]
    fn header_secrets_include_quoted_values() {
        assert_eq!(
            header_secrets(r#"MediaBrowser Token="abc", Client="""#),
            [r#"MediaBrowser Token="abc", Client="""#, "abc"]
        );
    }

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
