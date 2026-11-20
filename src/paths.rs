//! Locations on disk and small time helpers.

use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// The user's home directory. On Windows `USERPROFILE` wins so that PowerShell and
/// Git Bash sessions agree on the same location.
pub fn home_dir() -> PathBuf {
    let order: &[&str] = if cfg!(windows) {
        &["USERPROFILE", "HOME"]
    } else {
        &["HOME", "USERPROFILE"]
    };
    order
        .iter()
        .find_map(|k| env_path(k))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Where ojas keeps its config, stats and stored originals (`$OJAS_HOME` or `~/.ojas`).
pub fn ojas_home() -> PathBuf {
    env_path("OJAS_HOME").unwrap_or_else(|| home_dir().join(".ojas"))
}

/// Claude Code's config directory (`$CLAUDE_CONFIG_DIR` or `~/.claude`).
pub fn claude_dir() -> PathBuf {
    env_path("CLAUDE_CONFIG_DIR").unwrap_or_else(|| home_dir().join(".claude"))
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Format unix seconds as an ISO-8601 UTC timestamp, e.g. `2026-11-20T09:15:00Z`.
pub fn iso_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Howard Hinnant's days-to-civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_formatting() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_795_000_000), "2026-11-18T11:06:40Z");
    }
}
