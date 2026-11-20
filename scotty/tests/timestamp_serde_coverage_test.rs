//! Every serialized `jiff::Timestamp` field must go through `scotty_types::time`.
//!
//! A forgotten `ts(type)` or `schema(value_type)` annotation fails to compile,
//! but a forgotten `#[serde(with = "time::...")]` compiles and silently changes
//! the wire format (jiff trims fractional digits, chrono did not). This test
//! scans the workspace sources so a new field cannot slip through.

use std::fs;
use std::path::{Path, PathBuf};

const CRATES: [&str; 4] = ["scotty", "scotty-core", "scotty-types", "scottyctl"];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `name: Timestamp,` or `name: Option<Timestamp>,` (optionally `jiff::`).
fn is_timestamp_field(line: &str) -> bool {
    let line = line.trim();
    let Some((_, ty)) = line.split_once(": ") else {
        return false;
    };
    let ty = ty.trim_end_matches(',').replace("jiff::", "");
    !line.starts_with("//") && (ty == "Timestamp" || ty == "Option<Timestamp>")
}

/// `with = "..."` naming one of the `time` modules (`time::utc`,
/// `scotty_types::time::local_opt`, or plain `utc` inside `time.rs`).
fn uses_time_module(attrs: &str) -> bool {
    attrs.split("with = \"").skip(1).any(|rest| {
        let path = rest.split('"').next().unwrap_or_default();
        let module = path.rsplit("::").next().unwrap_or_default();
        ["utc", "utc_opt", "local", "local_opt"].contains(&module)
            && (path == module || path.ends_with(&format!("time::{module}")))
    })
}

/// A field or struct boundary: where the attributes of the next field start.
fn is_boundary(line: &str) -> bool {
    let line = line.trim();
    if line.starts_with("//") || line.starts_with('#') {
        return false;
    }
    line.ends_with('{') || (line.contains(": ") && line.ends_with(','))
}

/// Serde derives of the struct enclosing line `i`.
fn struct_derives(lines: &[&str], i: usize) -> String {
    let start = (0..i)
        .rev()
        .find(|&j| lines[j].contains("struct "))
        .unwrap_or(0);
    (0..start)
        .rev()
        .take_while(|&j| {
            let l = lines[j].trim();
            l.starts_with('#') || l.starts_with("//") || l.starts_with(')') || l.ends_with(',')
        })
        .map(|j| lines[j])
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn serialized_timestamps_use_the_time_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut files = Vec::new();
    for krate in CRATES {
        rust_files(&root.join(krate).join("src"), &mut files);
    }

    let mut checked = 0;
    let mut missing = Vec::new();
    for file in &files {
        let source = fs::read_to_string(file).unwrap();
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if !is_timestamp_field(line) {
                continue;
            }
            let derives = struct_derives(&lines, i);
            if !derives.contains("Serialize") && !derives.contains("Deserialize") {
                continue;
            }
            checked += 1;
            let attrs = (0..i)
                .rev()
                .take_while(|&j| !is_boundary(lines[j]))
                .map(|j| lines[j])
                .collect::<Vec<_>>()
                .join(" ");
            if !uses_time_module(&attrs) {
                missing.push(format!(
                    "{}:{}: {}",
                    file.strip_prefix(root).unwrap().display(),
                    i + 1,
                    line.trim()
                ));
            }
        }
    }

    // Guards the scanner itself: it must find the known fields.
    assert!(checked >= 14, "only {checked} timestamp fields found");
    assert!(
        missing.is_empty(),
        "serialized Timestamp fields without #[serde(with = \"time::...\")]:\n{}",
        missing.join("\n")
    );
}
