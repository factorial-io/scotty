//! Path-spec parsing for `app:cp`.
//!
//! Each argument is one of:
//! - `-`               → [`PathSpec::Stdio`]
//! - `app:service:path`/`app::path`/`app:path` → [`PathSpec::Remote`]
//! - anything else     → [`PathSpec::Local`]
//!
//! Heuristic: an argument is `Remote` iff it contains a `:`, AND the part
//! before the first `:` is neither a Windows drive letter (single ASCII
//! letter immediately followed by `:` — possibly trailed by `/` or `\`) nor
//! an existing local path.
//!
//! The Windows-drive rule is applied on every platform so that documented
//! examples behave identically on macOS, Linux, and Windows.

use std::path::{Path, PathBuf};

/// Parsed CLI argument for one side of `app:cp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSpec {
    /// Local filesystem path.
    Local(PathBuf),
    /// `-` sentinel meaning stdin (upload) or stdout (download).
    Stdio,
    /// Remote container path: `app[:service]:path`.
    Remote {
        app: String,
        service: Option<String>,
        path: String,
    },
}

impl PathSpec {
    /// Returns `true` if this spec refers to a container path.
    pub fn is_remote(&self) -> bool {
        matches!(self, PathSpec::Remote { .. })
    }
}

/// Parse one CLI argument into a [`PathSpec`].
pub fn parse_path_spec(arg: &str) -> PathSpec {
    parse_path_spec_with(arg, |p| Path::new(p).exists())
}

/// Internal parser injecting a `local_exists` probe so unit tests stay
/// hermetic.
pub(crate) fn parse_path_spec_with<F>(arg: &str, local_exists: F) -> PathSpec
where
    F: Fn(&str) -> bool,
{
    if arg == "-" {
        return PathSpec::Stdio;
    }

    let Some(first_colon) = arg.find(':') else {
        return PathSpec::Local(PathBuf::from(arg));
    };

    let head = &arg[..first_colon];

    // Windows drive letter: a single ASCII letter at position 0 followed by
    // `:`, and the next character (if any) is `/` or `\`.
    if is_windows_drive_prefix(arg) {
        return PathSpec::Local(PathBuf::from(arg));
    }

    // If the literal argument resolves to an existing local path on disk,
    // treat it as Local even if it contains a colon (rare but legal on Unix).
    if local_exists(arg) {
        return PathSpec::Local(PathBuf::from(arg));
    }

    // It's remote. The head must be non-empty (otherwise we'd have a
    // leading colon, which we treat as Local since `head` is the empty
    // app id).
    if head.is_empty() {
        return PathSpec::Local(PathBuf::from(arg));
    }

    let rest = &arg[first_colon + 1..];

    // `app::path` — empty service.
    if let Some(stripped) = rest.strip_prefix(':') {
        return PathSpec::Remote {
            app: head.to_string(),
            service: None,
            path: stripped.to_string(),
        };
    }

    // `app:service:path` vs `app:path`. We follow docker-cp: a second colon
    // promotes the middle segment to `service`. The path itself may legally
    // contain colons after that point.
    match rest.find(':') {
        Some(second_colon) => {
            let service = &rest[..second_colon];
            let path = &rest[second_colon + 1..];
            PathSpec::Remote {
                app: head.to_string(),
                service: if service.is_empty() {
                    None
                } else {
                    Some(service.to_string())
                },
                path: path.to_string(),
            }
        }
        None => PathSpec::Remote {
            app: head.to_string(),
            service: None,
            path: rest.to_string(),
        },
    }
}

/// `true` if `s` begins with a Windows-style drive prefix such as `C:`,
/// `C:/`, or `C:\`.
fn is_windows_drive_prefix(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    if !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return false;
    }
    // Either the string is exactly "X:" or the next char is a separator.
    matches!(bytes.get(2), None | Some(&b'/') | Some(&b'\\'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn never_exists(_: &str) -> bool {
        false
    }

    #[test]
    fn parses_stdio_dash() {
        assert_eq!(parse_path_spec("-"), PathSpec::Stdio);
    }

    #[test]
    fn parses_plain_local_path() {
        assert_eq!(
            parse_path_spec_with("./Cargo.toml", never_exists),
            PathSpec::Local(PathBuf::from("./Cargo.toml"))
        );
        assert_eq!(
            parse_path_spec_with("foo/bar", never_exists),
            PathSpec::Local(PathBuf::from("foo/bar"))
        );
    }

    #[test]
    fn parses_remote_app_service_path() {
        assert_eq!(
            parse_path_spec_with("myapp:web:/var/log/app.log", never_exists),
            PathSpec::Remote {
                app: "myapp".into(),
                service: Some("web".into()),
                path: "/var/log/app.log".into(),
            }
        );
    }

    #[test]
    fn parses_remote_with_empty_service() {
        assert_eq!(
            parse_path_spec_with("myapp::/etc/hostname", never_exists),
            PathSpec::Remote {
                app: "myapp".into(),
                service: None,
                path: "/etc/hostname".into(),
            }
        );
    }

    #[test]
    fn parses_remote_without_service_segment() {
        assert_eq!(
            parse_path_spec_with("myapp:/etc/hostname", never_exists),
            PathSpec::Remote {
                app: "myapp".into(),
                service: None,
                path: "/etc/hostname".into(),
            }
        );
    }

    #[test]
    fn windows_drive_letter_is_local() {
        assert_eq!(
            parse_path_spec_with("C:/Users/me/file.txt", never_exists),
            PathSpec::Local(PathBuf::from("C:/Users/me/file.txt"))
        );
        assert_eq!(
            parse_path_spec_with("D:\\data\\dump.sql", never_exists),
            PathSpec::Local(PathBuf::from("D:\\data\\dump.sql"))
        );
        assert_eq!(
            parse_path_spec_with("Z:", never_exists),
            PathSpec::Local(PathBuf::from("Z:"))
        );
    }

    #[test]
    fn existing_local_path_with_colon_is_local() {
        // Simulate that "weird:name.txt" exists on disk.
        let arg = "weird:name.txt";
        let exists = |p: &str| p == arg;
        assert_eq!(
            parse_path_spec_with(arg, exists),
            PathSpec::Local(PathBuf::from(arg))
        );
    }

    #[test]
    fn leading_colon_is_local() {
        assert_eq!(
            parse_path_spec_with(":foo", never_exists),
            PathSpec::Local(PathBuf::from(":foo"))
        );
    }

    /// Exactly one side must be remote; both-remote or both-local must be
    /// rejected by the caller. We exercise the pure validation helper here.
    #[test]
    fn reject_both_remote_and_both_local() {
        use super::super::validate_endpoints;
        let r1 = parse_path_spec_with("app1:svc:/a", never_exists);
        let r2 = parse_path_spec_with("app2:svc:/b", never_exists);
        let l1 = parse_path_spec_with("./a", never_exists);
        let l2 = parse_path_spec_with("./b", never_exists);
        let stdio = parse_path_spec("-");

        assert!(validate_endpoints(&r1, &r2).is_err());
        assert!(validate_endpoints(&l1, &l2).is_err());
        assert!(validate_endpoints(&stdio, &l1).is_err());
        assert!(validate_endpoints(&r1, &l1).is_ok());
        assert!(validate_endpoints(&l1, &r1).is_ok());
        assert!(validate_endpoints(&stdio, &r1).is_ok());
        assert!(validate_endpoints(&r1, &stdio).is_ok());
    }
}
