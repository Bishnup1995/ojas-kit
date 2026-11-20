//! git status / diff / log.

const LOCKFILES: &[&str] = &[
    "package-lock.json",
    "npm-shrinkwrap.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "bun.lockb",
    "Cargo.lock",
    "poetry.lock",
    "uv.lock",
    "Pipfile.lock",
    "Gemfile.lock",
    "composer.lock",
    "go.sum",
];

/// Drop the `(use "git ...")` hints and cap very long file lists per section.
pub fn status(text: &str) -> String {
    const PER_SECTION: usize = 30;
    let mut out = Vec::new();
    let (mut count, mut hidden) = (0usize, 0usize);
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("(use \"git")
            || t.starts_with("(commit or discard")
            || t.starts_with("(create/copy files")
            || t.starts_with("(fix conflicts")
            || t.starts_with("(all conflicts fixed")
            || t.starts_with("no changes added to commit")
        {
            continue;
        }
        if line.starts_with('\t') {
            count += 1;
            if count > PER_SECTION {
                hidden += 1;
                continue;
            }
        } else {
            if hidden > 0 {
                out.push(format!("\t… and {hidden} more"));
                hidden = 0;
            }
            count = 0;
        }
        out.push(line.to_string());
    }
    if hidden > 0 {
        out.push(format!("\t… and {hidden} more"));
    }
    out.join("\n")
}

/// Keep file headers, hunk headers and changed lines with one line of context.
/// Lockfiles collapse to a count. Adds a one-line summary when several files changed.
pub fn diff(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("diff --git "))
        .map(|(i, _)| i)
        .collect();
    if starts.is_empty() {
        return text.to_string();
    }
    let mut out: Vec<String> = lines[..starts[0]].iter().map(|s| s.to_string()).collect();
    starts.push(lines.len());
    let (mut adds, mut dels) = (0, 0);
    let mut body = Vec::new();
    for w in starts.windows(2) {
        let (a, d, rendered) = file_chunk(&lines[w[0]..w[1]]);
        adds += a;
        dels += d;
        body.extend(rendered);
    }
    let files = starts.len() - 1;
    if files > 1 {
        out.push(format!("[{files} files changed, +{adds} -{dels}]"));
    }
    out.extend(body);
    out.join("\n")
}

fn is_change(l: &str) -> bool {
    l.starts_with('+') || l.starts_with('-')
}

fn file_chunk(chunk: &[&str]) -> (usize, usize, Vec<String>) {
    const PER_FILE: usize = 150;
    let header = chunk[0];
    let path = header.rsplit(" b/").next().unwrap_or(header);
    let name = path.rsplit('/').next().unwrap_or(path);

    let mut i = 1;
    let mut meta = Vec::new();
    while i < chunk.len() && !chunk[i].starts_with("@@") {
        let l = chunk[i];
        if !(l.starts_with("index ") || l.starts_with("--- ") || l.starts_with("+++ ")) {
            meta.push(l.to_string());
        }
        i += 1;
    }
    let body = &chunk[i..];
    let adds = body.iter().filter(|l| l.starts_with('+')).count();
    let dels = body.iter().filter(|l| l.starts_with('-')).count();

    let mut out = vec![header.to_string()];
    out.extend(meta);
    if LOCKFILES.contains(&name) {
        out.push(format!("  (lockfile: +{adds} -{dels} lines hidden)"));
        return (adds, dels, out);
    }
    let mut emitted = 0;
    for (j, l) in body.iter().enumerate() {
        let keep = l.starts_with("@@")
            || is_change(l)
            || l.starts_with('\\')
            || (j > 0 && is_change(body[j - 1]))
            || body.get(j + 1).is_some_and(|n| is_change(n));
        if !keep {
            continue;
        }
        if emitted >= PER_FILE {
            let rest = body[j..].iter().filter(|l| is_change(l)).count();
            out.push(format!("  … {rest} more changed lines in this file"));
            break;
        }
        out.push(l.to_string());
        emitted += 1;
    }
    (adds, dels, out)
}

struct Commit {
    hash: String,
    refs: String,
    author: String,
    date: String,
    subject: String,
    extra: usize,
    other: Vec<String>,
}

impl Commit {
    fn render(&self, out: &mut Vec<String>) {
        let mut line = self.hash.clone();
        if !self.refs.is_empty() {
            line.push(' ');
            line.push_str(&self.refs);
        }
        if !self.date.is_empty() {
            line.push(' ');
            line.push_str(&self.date);
        }
        if !self.author.is_empty() {
            line.push(' ');
            line.push_str(&self.author);
            line.push(':');
        }
        line.push(' ');
        line.push_str(&self.subject);
        if self.extra > 0 {
            line.push_str(&format!(" (+{} body lines)", self.extra));
        }
        out.push(line);
        out.extend(self.other.iter().map(|o| format!("  {}", o.trim())));
    }
}

fn short_date(d: &str) -> String {
    let p: Vec<&str> = d.split_whitespace().collect();
    if p.first().is_some_and(|x| x.contains('-')) {
        return p[0].to_string();
    }
    if p.len() >= 5 {
        return format!("{} {} {}", p[1], p[2], p[4]);
    }
    d.to_string()
}

/// Turn the default multi-line `git log` format into one line per commit.
/// Logs with patches (`-p`) are left to the generic truncation.
pub fn log(text: &str) -> String {
    let is_commit = |l: &str| {
        l.strip_prefix("commit ").is_some_and(|rest| {
            let h = rest.split_whitespace().next().unwrap_or("");
            h.len() >= 7 && h.chars().all(|c| c.is_ascii_hexdigit())
        })
    };
    if !text.lines().any(is_commit) || text.contains("\ndiff --git ") {
        return text.to_string();
    }
    let mut out = Vec::new();
    let mut cur: Option<Commit> = None;
    for line in text.lines() {
        if is_commit(line) {
            if let Some(c) = cur.take() {
                c.render(&mut out);
            }
            let rest = &line["commit ".len()..];
            let (h, refs) = rest.split_once(' ').unwrap_or((rest, ""));
            cur = Some(Commit {
                hash: h.chars().take(10).collect(),
                refs: refs.trim().to_string(),
                author: String::new(),
                date: String::new(),
                subject: String::new(),
                extra: 0,
                other: Vec::new(),
            });
            continue;
        }
        let Some(c) = cur.as_mut() else {
            out.push(line.to_string());
            continue;
        };
        if let Some(a) = line.strip_prefix("Author:") {
            c.author = a.trim().split(" <").next().unwrap_or("").to_string();
        } else if let Some(d) = line.strip_prefix("Date:") {
            c.date = short_date(d.trim());
        } else if line.starts_with("Merge:") || line.trim().is_empty() {
        } else if line.starts_with("    ") || line.starts_with('\t') {
            if c.subject.is_empty() {
                c.subject = line.trim().to_string();
            } else {
                c.extra += 1;
            }
        } else {
            c.other.push(line.to_string());
        }
    }
    if let Some(c) = cur {
        c.render(&mut out);
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_one_line_per_commit() {
        let raw = "commit 0123456789abcdef0123456789abcdef01234567 (HEAD -> main)\nAuthor: Ada Lovelace <ada@example.com>\nDate:   Wed Nov 19 10:00:00 2026 +0530\n\n    Add engine\n\n    Longer body\n    more body\n\ncommit fedcba9876543210fedcba9876543210fedcba98\nAuthor: Bob <b@example.com>\nDate:   Tue Nov 18 09:00:00 2026 +0000\n\n    Init\n";
        assert_eq!(
            log(raw),
            "0123456789 (HEAD -> main) Nov 19 2026 Ada Lovelace: Add engine (+2 body lines)\nfedcba9876 Nov 18 2026 Bob: Init"
        );
    }

    #[test]
    fn diff_trims_context_and_lockfiles() {
        let raw = "diff --git a/src/a.rs b/src/a.rs\nindex 111..222 100644\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,7 +1,7 @@\n ctx1\n ctx2\n ctx3\n-old\n+new\n ctx4\n ctx5\ndiff --git a/Cargo.lock b/Cargo.lock\n--- a/Cargo.lock\n+++ b/Cargo.lock\n@@ -1,2 +1,2 @@\n-a\n+b\n";
        let out = diff(raw);
        assert!(out.starts_with("[2 files changed, +2 -2]"));
        assert!(out.contains(" ctx3\n-old\n+new\n ctx4"));
        assert!(!out.contains("ctx1"));
        assert!(!out.contains("index 111"));
        assert!(out.contains("(lockfile: +1 -1 lines hidden)"));
    }

    #[test]
    fn status_drops_hints() {
        let raw = "On branch main\nChanges not staged for commit:\n  (use \"git add <file>...\" to update what will be committed)\n\tmodified:   a.rs\n";
        assert_eq!(
            status(raw),
            "On branch main\nChanges not staged for commit:\n\tmodified:   a.rs"
        );
    }
}
