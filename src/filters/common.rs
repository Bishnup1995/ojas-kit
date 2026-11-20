//! Building blocks shared by every filter: cleanup, repeat collapsing, and
//! error-preserving truncation.

use regex::Regex;
use std::sync::LazyLock;

static ANSI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[@-Z\\-_]").unwrap()
});
static NUMBERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"0x[0-9a-fA-F]+|[0-9a-fA-F]{7,}|\d+").unwrap());
static ERROR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:error|errors|err!|fail|failed|failure|failures|failing|panic|panicked|exception|fatal|traceback|assertion|segfault|denied|cannot find|undefined reference)\b|✕|✗|✘|×",
    )
    .unwrap()
});
static WARNING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:warn|warning|warnings|deprecated)\b").unwrap());

pub fn is_error(line: &str) -> bool {
    ERROR.is_match(line)
}

pub fn is_warning(line: &str) -> bool {
    WARNING.is_match(line)
}

/// Strip ANSI escapes, resolve `\r` progress redraws, trim trailing space, and
/// squeeze runs of blank lines.
pub fn clean(raw: &str) -> String {
    let s = ANSI.replace_all(raw, "");
    let mut out = String::with_capacity(s.len());
    let mut blanks = 0;
    for line in s.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let line = match line.rfind('\r') {
            Some(i) => &line[i + 1..],
            None => line,
        };
        let line = line.trim_end();
        if line.is_empty() {
            blanks += 1;
            if blanks > 1 {
                continue;
            }
        } else {
            blanks = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim_matches('\n').to_string()
}

/// Shorten very long lines (minified code, base64) to their head and tail.
pub fn shorten_line(line: &str, max: usize) -> String {
    let n = line.chars().count();
    if n <= max {
        return line.to_string();
    }
    let head: String = line.chars().take(max * 2 / 3).collect();
    let tail: String = line.chars().skip(n - max / 4).collect();
    let cut = n - head.chars().count() - tail.chars().count();
    format!("{head} …[{cut} chars]… {tail}")
}

fn template(line: &str) -> String {
    NUMBERS.replace_all(line, "#").into_owned()
}

/// Collapse exact repeats into `line [×N]`. With `similar`, also collapse runs of 4+
/// lines that differ only in numbers or hashes, keeping the first and last. Error
/// lines are never merged into a "similar" run.
pub fn collapse_repeats(lines: Vec<String>, similar: bool) -> Vec<String> {
    let templates: Vec<Option<String>> = if similar {
        lines
            .iter()
            .map(|l| (!l.trim().is_empty() && !is_error(l)).then(|| template(l)))
            .collect()
    } else {
        Vec::new()
    };
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let mut j = i + 1;
        while j < lines.len() && lines[j] == lines[i] {
            j += 1;
        }
        if j - i >= 2 && !lines[i].trim().is_empty() {
            out.push(format!("{}  [×{}]", lines[i], j - i));
            i = j;
            continue;
        }
        if let Some(Some(t)) = templates.get(i) {
            let mut k = i + 1;
            while k < lines.len() && templates[k].as_ref() == Some(t) {
                k += 1;
            }
            if k - i >= 4 {
                out.push(lines[i].clone());
                out.push(format!("  … {} similar lines …", k - i - 2));
                out.push(lines[k - 1].clone());
                i = k;
                continue;
            }
        }
        out.push(lines[i].clone());
        i += 1;
    }
    out
}

/// Keep at most about `max` lines: the head, the tail, and error lines (with one
/// line of context) from the middle, then warnings. Error lines get their own
/// budget of `max` extra lines so failures are not crowded out. Gaps become a
/// marker that says how many lines, errors and warnings were dropped.
#[allow(clippy::needless_range_loop)] // index loops over `keep` and `lines` together read clearer
pub fn truncate(lines: Vec<String>, max: usize) -> Vec<String> {
    let n = lines.len();
    if n <= max || max < 10 {
        return lines;
    }
    let head = max * 3 / 10;
    let tail = max * 3 / 10;
    let mut keep = vec![false; n];
    keep[..head].iter_mut().for_each(|k| *k = true);
    keep[n - tail..].iter_mut().for_each(|k| *k = true);

    let mut err_budget = max;
    for i in head..n - tail {
        if err_budget == 0 {
            break;
        }
        if is_error(&lines[i]) {
            for j in i.saturating_sub(1)..=(i + 1).min(n - 1) {
                if !keep[j] && err_budget > 0 {
                    keep[j] = true;
                    err_budget -= 1;
                }
            }
        }
    }
    let mut budget = max - head - tail;
    for i in head..n - tail {
        if budget == 0 {
            break;
        }
        if !keep[i] && is_warning(&lines[i]) {
            keep[i] = true;
            budget -= 1;
        }
    }

    let mut out = Vec::with_capacity(max * 2);
    let mut i = 0;
    while i < n {
        if keep[i] {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }
        let start = i;
        let (mut errs, mut warns) = (0, 0);
        while i < n && !keep[i] {
            if is_error(&lines[i]) {
                errs += 1;
            } else if is_warning(&lines[i]) {
                warns += 1;
            }
            i += 1;
        }
        let mut msg = format!("… {} lines omitted", i - start);
        if errs > 0 || warns > 0 {
            msg.push_str(&format!(" ({errs} error-like, {warns} warning-like)"));
        }
        msg.push_str(" …");
        out.push(msg);
    }
    out
}

/// The standard tail of every text filter.
pub fn finalize(text: &str, max_lines: usize, similar: bool) -> String {
    let lines: Vec<String> = text.lines().map(|l| shorten_line(l, 400)).collect();
    let lines = collapse_repeats(lines, similar);
    truncate(lines, max_lines).join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_strips_ansi_and_progress() {
        let raw = "\x1b[32mok\x1b[0m\r\n10%\r50%\r100%\n\n\n\nend  \n";
        assert_eq!(clean(raw), "ok\n100%\n\nend");
    }

    #[test]
    fn collapse_exact_and_similar() {
        let lines: Vec<String> = [
            "a", "a", "a", "req 1 ok", "req 2 ok", "req 3 ok", "req 4 ok", "z",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let out = collapse_repeats(lines, true);
        assert_eq!(
            out,
            vec![
                "a  [×3]",
                "req 1 ok",
                "  … 2 similar lines …",
                "req 4 ok",
                "z"
            ]
        );
    }

    #[test]
    fn similar_never_hides_errors() {
        let lines: Vec<String> = (1..=6).map(|i| format!("error at line {i}")).collect();
        let out = collapse_repeats(lines.clone(), true);
        assert_eq!(out, lines);
    }

    #[test]
    fn truncate_keeps_errors_from_the_middle() {
        let mut lines: Vec<String> = (0..1000).map(|i| format!("line {i}")).collect();
        lines[500] = "ERROR: disk full".into();
        let out = truncate(lines, 50);
        assert!(out.iter().any(|l| l == "ERROR: disk full"));
        assert!(out
            .iter()
            .any(|l| l.starts_with("… ") && l.contains("lines omitted")));
        assert!(out.len() < 70);
    }
}
