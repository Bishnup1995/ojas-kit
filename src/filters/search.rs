//! grep / rg / ag output: group matches by file and cap per-file and total counts.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

use super::common::{finalize, shorten_line};

static WITH_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^((?:[A-Za-z]:)?[^:\n]*?):(\d+)[:\-](.*)$").unwrap());
static NO_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^((?:[A-Za-z]:)?[^:\s][^:\n]*?):(.*)$").unwrap());

const PER_FILE: usize = 8;
const MAX_FILES: usize = 40;

fn looks_like_path(s: &str) -> bool {
    !s.contains("  ") && (s.contains('/') || s.contains('\\') || s.contains('.'))
}

pub fn compress(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text.lines().filter(|l| *l != "--").collect();
    if lines.is_empty() {
        return text.to_string();
    }
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut other: Vec<&str> = Vec::new();
    for l in &lines {
        let parsed = if let Some(c) = WITH_LINE.captures(l) {
            Some((c[1].to_string(), format!("{}: {}", &c[2], c[3].trim())))
        } else {
            NO_LINE
                .captures(l)
                .filter(|c| looks_like_path(&c[1]))
                .map(|c| (c[1].to_string(), c[2].trim().to_string()))
        };
        match parsed {
            Some((file, entry)) => {
                let i = *index.entry(file.clone()).or_insert_with(|| {
                    groups.push((file, Vec::new()));
                    groups.len() - 1
                });
                groups[i].1.push(entry);
            }
            None => other.push(l),
        }
    }
    let parsed: usize = groups.iter().map(|g| g.1.len()).sum();
    // Not grep-shaped (e.g. `grep -l`): fall back to generic handling.
    if parsed * 10 < lines.len() * 6 {
        return finalize(text, max_lines, false);
    }

    let mut out = vec![format!("{parsed} matches in {} files", groups.len())];
    let mut shown = 0;
    for (file, entries) in &groups {
        if shown >= MAX_FILES || out.len() >= max_lines {
            break;
        }
        out.push(format!("{file} ({})", entries.len()));
        for e in entries.iter().take(PER_FILE) {
            out.push(format!("  {}", shorten_line(e, 200)));
        }
        if entries.len() > PER_FILE {
            out.push(format!("  … +{} more", entries.len() - PER_FILE));
        }
        shown += 1;
    }
    if shown < groups.len() {
        let rest: usize = groups[shown..].iter().map(|g| g.1.len()).sum();
        out.push(format!(
            "… +{rest} more matches in {} more files",
            groups.len() - shown
        ));
    }
    for o in other.iter().take(10) {
        out.push(o.to_string());
    }
    if other.len() > 10 {
        out.push(format!("… +{} other lines", other.len() - 10));
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_by_file() {
        let mut raw = String::new();
        for i in 1..=12 {
            raw.push_str(&format!("src/a.rs:{i}:    let x{i} = foo();\n"));
        }
        raw.push_str("src/b.rs:3:foo()\n");
        let out = compress(&raw, 150);
        assert!(out.starts_with("13 matches in 2 files\nsrc/a.rs (12)\n  1: let x1 = foo();"));
        assert!(out.contains("  … +4 more"));
        assert!(out.contains("src/b.rs (1)\n  3: foo()"));
    }

    #[test]
    fn windows_paths() {
        let raw = "C:\\repo\\a.rs:10:hit\nC:\\repo\\a.rs:20:hit\nC:\\repo\\b.rs:1:hit\n";
        let out = compress(raw, 150);
        assert!(out.contains("C:\\repo\\a.rs (2)"));
    }
}
