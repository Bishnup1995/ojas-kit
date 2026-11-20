//! Content-addressed store for original (uncompressed) command output.
//!
//! Compression is only safe because nothing is lost: every original is written to
//! `~/.ojas/store/<id>.txt` and can be read back with `ojas get <id>`.

use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crate::paths;

fn dir() -> PathBuf {
    paths::ojas_home().join("store")
}

/// Store `content` and return its id (first 12 hex chars of its SHA-256).
pub fn put(content: &str) -> io::Result<String> {
    let hash = Sha256::digest(content.as_bytes());
    let id: String = hash.iter().take(6).map(|b| format!("{b:02x}")).collect();
    let d = dir();
    fs::create_dir_all(&d)?;
    let path = d.join(format!("{id}.txt"));
    if !path.exists() {
        fs::write(&path, content)?;
    }
    Ok(id)
}

/// Read a stored original by id or unique id prefix (at least 4 hex chars).
pub fn get(id: &str) -> io::Result<String> {
    let id = id.trim().to_ascii_lowercase();
    if id.len() < 4 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("'{id}' is not a valid id (expected hex, at least 4 characters)"),
        ));
    }
    let d = dir();
    let exact = d.join(format!("{id}.txt"));
    if exact.is_file() {
        return fs::read_to_string(exact);
    }
    let mut matches = Vec::new();
    if let Ok(entries) = fs::read_dir(&d) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with(&id) && name.ends_with(".txt") {
                matches.push(e.path());
            }
        }
    }
    match matches.len() {
        1 => fs::read_to_string(&matches[0]),
        0 => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no stored output with id {id} (originals are kept for a limited number of days)"
            ),
        )),
        n => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("id prefix {id} matches {n} stored outputs; use more characters"),
        )),
    }
}

/// Delete originals older than `days`. Runs at most once a day.
pub fn prune_if_due(days: u64) {
    let d = dir();
    let stamp = d.join(".last-prune");
    let due = fs::metadata(&stamp)
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().unwrap_or_default() > Duration::from_secs(86_400))
        .unwrap_or(true);
    if !due || fs::create_dir_all(&d).is_err() {
        return;
    }
    let _ = fs::write(&stamp, b"");
    let max_age = Duration::from_secs(days.max(1) * 86_400);
    let Ok(entries) = fs::read_dir(&d) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "txt") {
            if let Ok(modified) = e.metadata().and_then(|m| m.modified()) {
                if modified.elapsed().unwrap_or_default() > max_age {
                    let _ = fs::remove_file(p);
                }
            }
        }
    }
}
