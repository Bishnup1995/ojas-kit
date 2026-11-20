//! Decide how to rewrite a shell command so that noisy parts run under `ojas run --`.
//!
//! Safety rule: only rewrite plain simple commands. Prefixing a simple command with
//! `ojas run --` leaves the shell's own parsing (quotes, globs, `~`, `$VAR`) unchanged,
//! so the program receives exactly the same arguments. Anything we cannot reason about
//! (pipes, redirections, subshells, command substitution, heredocs, env assignments)
//! is left alone. Chains joined by `&&`, `||` or `;` are rewritten segment by segment.

use std::path::Path;

use crate::{code, filters};

enum Piece<'a> {
    Cmd(&'a str),
    Sep(&'a str),
}

/// Like [`rewrite`], plus: a plain `cat <one source file>` of more than
/// `read_min_lines` lines becomes `ojas read <file>` (an outline). `cwd` is the
/// directory relative paths are resolved against (the session's cwd).
pub fn rewrite_ctx(
    cmd: &str,
    bin: &str,
    cwd: Option<&Path>,
    read_min_lines: usize,
) -> Option<String> {
    rewrite_cat(cmd, bin, cwd, read_min_lines).or_else(|| rewrite(cmd, bin))
}

fn rewrite_cat(cmd: &str, bin: &str, cwd: Option<&Path>, read_min_lines: usize) -> Option<String> {
    // Exactly one simple command, with no expansion we can't resolve ourselves.
    if split(cmd)?.len() != 1 || cmd.contains(['$', '~', '*', '?', '[', '{']) {
        return None;
    }
    let w = words(cmd);
    if w.len() != 2 || w[0] != "cat" || w[1].starts_with('-') {
        return None;
    }
    let rel = Path::new(&w[1]);
    code::lang_for(rel)?;
    let full = match cwd {
        Some(c) if rel.is_relative() => c.join(rel),
        _ => rel.to_path_buf(),
    };
    let meta = std::fs::metadata(&full).ok()?;
    if !meta.is_file() || meta.len() > 20 * 1024 * 1024 {
        return None;
    }
    let bytes = std::fs::read(&full).ok()?;
    if bytes.iter().filter(|b| **b == b'\n').count() <= read_min_lines {
        return None;
    }
    let arg = cmd.trim()["cat".len()..].trim();
    Some(format!("{bin} read {arg}"))
}

/// Returns the rewritten command, or `None` to leave it untouched.
pub fn rewrite(cmd: &str, bin: &str) -> Option<String> {
    let pieces = split(cmd)?;
    let mut changed = false;
    let mut out = String::with_capacity(cmd.len() + 32);
    for piece in &pieces {
        match piece {
            Piece::Sep(s) => out.push_str(s),
            Piece::Cmd(text) => {
                let words = words(text);
                let Some(first) = words.first() else {
                    out.push_str(text);
                    continue;
                };
                if filters::program_name(first) == "ojas" || filters::program_name(first) == "tsave"
                {
                    return None; // already wrapped somewhere: leave the whole thing alone
                }
                if !is_assignment(first) && filters::classify(&words).is_some() {
                    let trimmed = text.trim();
                    let lead = &text[..text.len() - text.trim_start().len()];
                    let trail = &text[lead.len() + trimmed.len()..];
                    out.push_str(lead);
                    out.push_str(bin);
                    out.push_str(" run -- ");
                    out.push_str(trimmed);
                    out.push_str(trail);
                    changed = true;
                } else {
                    out.push_str(text);
                }
            }
        }
    }
    changed.then_some(out)
}

#[derive(PartialEq)]
enum Quote {
    None,
    Single,
    Double,
}

/// Split on top-level `&&`, `||` and `;`. Returns `None` for anything unsupported.
fn split(cmd: &str) -> Option<Vec<Piece<'_>>> {
    let b = cmd.as_bytes();
    let mut pieces = Vec::new();
    let (mut i, mut start) = (0usize, 0usize);
    let mut q = Quote::None;
    while i < b.len() {
        let c = b[i];
        match q {
            Quote::Single => {
                if c == b'\'' {
                    q = Quote::None;
                }
            }
            Quote::Double => match c {
                b'\\' => i += 1,
                b'"' => q = Quote::None,
                b'`' => return None,
                b'$' if b.get(i + 1) == Some(&b'(') => return None,
                _ => {}
            },
            Quote::None => match c {
                b'\\' => i += 1,
                b'\'' => q = Quote::Single,
                b'"' => q = Quote::Double,
                b'`' | b'(' | b')' | b'<' | b'\n' | b'\r' => return None,
                b'$' if b.get(i + 1) == Some(&b'(') => return None,
                b'#' if i == 0 || b[i - 1].is_ascii_whitespace() => return None,
                b'>' => {
                    // Allow exactly `2>&1`; any other redirection is unsupported.
                    let is_2_to_1 = i >= 1
                        && b[i - 1] == b'2'
                        && (i < 2 || b[i - 2].is_ascii_whitespace())
                        && b[i + 1..].starts_with(b"&1");
                    if !is_2_to_1 {
                        return None;
                    }
                    i += 3;
                    continue;
                }
                b'&' | b'|' => {
                    if b.get(i + 1) == Some(&c) {
                        pieces.push(Piece::Cmd(&cmd[start..i]));
                        pieces.push(Piece::Sep(&cmd[i..i + 2]));
                        i += 2;
                        start = i;
                        continue;
                    }
                    return None; // pipe or background job
                }
                b';' => {
                    pieces.push(Piece::Cmd(&cmd[start..i]));
                    pieces.push(Piece::Sep(&cmd[i..i + 1]));
                    i += 1;
                    start = i;
                    continue;
                }
                _ => {}
            },
        }
        i += 1;
    }
    if q != Quote::None {
        return None;
    }
    pieces.push(Piece::Cmd(&cmd[start..]));
    Some(pieces)
}

/// Shell-like word splitting, good enough to read the first few words of a command.
pub fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut has = false;
    let mut q: Option<char> = None;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match (q, c) {
            (None, c) if c.is_whitespace() => {
                if has {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            (None, '\'' | '"') => {
                q = Some(c);
                has = true;
            }
            (Some(qc), c) if c == qc => q = None,
            (None, '\\') | (Some('"'), '\\') => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                    has = true;
                }
            }
            (_, c) => {
                cur.push(c);
                has = true;
            }
        }
    }
    if has {
        out.push(cur);
    }
    out
}

fn is_assignment(w: &str) -> bool {
    match w.split_once('=') {
        Some((name, _)) => {
            !name.is_empty()
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !name.starts_with(|c: char| c.is_ascii_digit())
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rw(cmd: &str) -> Option<String> {
        rewrite(cmd, "ojas")
    }

    #[test]
    fn wraps_simple_commands() {
        assert_eq!(rw("git status").as_deref(), Some("ojas run -- git status"));
        assert_eq!(
            rw("  cargo test --all  ").as_deref(),
            Some("  ojas run -- cargo test --all  ")
        );
        assert_eq!(
            rw(r#"rg -n "a|b" src"#).as_deref(),
            Some(r#"ojas run -- rg -n "a|b" src"#)
        );
        assert_eq!(
            rw("cargo test 2>&1").as_deref(),
            Some("ojas run -- cargo test 2>&1")
        );
    }

    #[test]
    fn wraps_chain_segments() {
        assert_eq!(
            rw("cd web && npm test").as_deref(),
            Some("cd web && ojas run -- npm test")
        );
        assert_eq!(
            rw("cargo build; cargo test || echo failed").as_deref(),
            Some("ojas run -- cargo build; ojas run -- cargo test || echo failed")
        );
    }

    #[test]
    fn leaves_unsafe_or_unknown_alone() {
        for cmd in [
            "cargo test | tail -20",
            "git log > out.txt",
            "git diff $(git merge-base main HEAD)",
            "echo `git status`",
            "(cd x && git status)",
            "npm test &",
            "OJAS_RAW=1 git status",
            "FOO=bar cargo test",
            "cat <<EOF\nx\nEOF",
            "git status # comment",
            "git commit -m 'msg'",
            "rm -rf build",
            "ojas run -- git status",
            "ls 'unterminated",
            "",
        ] {
            assert_eq!(rw(cmd), None, "should not rewrite: {cmd}");
        }
    }

    #[test]
    fn cat_of_large_source_becomes_outline_read() {
        let dir = std::env::temp_dir().join(format!("ojas-cat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let big: String = (0..400).map(|i| format!("fn f{i}() {{}}\n")).collect();
        std::fs::write(dir.join("big.rs"), &big).unwrap();
        std::fs::write(dir.join("small.rs"), "fn a() {}\n").unwrap();
        std::fs::write(dir.join("big.txt"), &big).unwrap();
        let rw = |c: &str| rewrite_ctx(c, "ojas", Some(&dir), 300);
        assert_eq!(rw("cat big.rs").as_deref(), Some("ojas read big.rs"));
        assert_eq!(rw("cat 'big.rs'").as_deref(), Some("ojas read 'big.rs'"));
        for cmd in [
            "cat small.rs",
            "cat big.txt",
            "cat -n big.rs",
            "cat big.rs small.rs",
            "cat missing.rs",
            "cat *.rs",
            "cat big.rs | head",
            "cd x && cat big.rs",
        ] {
            assert_eq!(rw(cmd), None, "should not rewrite: {cmd}");
        }
        // Other commands still follow the normal rules.
        assert_eq!(rw("cargo test").as_deref(), Some("ojas run -- cargo test"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn word_splitting() {
        assert_eq!(
            words(r#"git log --format="%h %s" -n 3"#),
            vec!["git", "log", "--format=%h %s", "-n", "3"]
        );
        assert_eq!(words("a 'b c' d\\ e"), vec!["a", "b c", "d e"]);
    }
}
