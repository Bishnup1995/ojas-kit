//! Decide how to rewrite a shell command so that noisy parts run under `tsave run --`.
//!
//! Safety rule: only rewrite plain simple commands. Prefixing a simple command with
//! `tsave run --` leaves the shell's own parsing (quotes, globs, `~`, `$VAR`) unchanged,
//! so the program receives exactly the same arguments. Anything we cannot reason about
//! (pipes, redirections, subshells, command substitution, heredocs, env assignments)
//! is left alone. Chains joined by `&&`, `||` or `;` are rewritten segment by segment.

use crate::filters;

enum Piece<'a> {
    Cmd(&'a str),
    Sep(&'a str),
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
                if filters::program_name(first) == "tsave" {
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
        rewrite(cmd, "tsave")
    }

    #[test]
    fn wraps_simple_commands() {
        assert_eq!(rw("git status").as_deref(), Some("tsave run -- git status"));
        assert_eq!(
            rw("  cargo test --all  ").as_deref(),
            Some("  tsave run -- cargo test --all  ")
        );
        assert_eq!(
            rw(r#"rg -n "a|b" src"#).as_deref(),
            Some(r#"tsave run -- rg -n "a|b" src"#)
        );
        assert_eq!(
            rw("cargo test 2>&1").as_deref(),
            Some("tsave run -- cargo test 2>&1")
        );
    }

    #[test]
    fn wraps_chain_segments() {
        assert_eq!(
            rw("cd web && npm test").as_deref(),
            Some("cd web && tsave run -- npm test")
        );
        assert_eq!(
            rw("cargo build; cargo test || echo failed").as_deref(),
            Some("tsave run -- cargo build; tsave run -- cargo test || echo failed")
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
            "TSAVE_RAW=1 git status",
            "FOO=bar cargo test",
            "cat <<EOF\nx\nEOF",
            "git status # comment",
            "git commit -m 'msg'",
            "rm -rf build",
            "tsave run -- git status",
            "ls 'unterminated",
            "",
        ] {
            assert_eq!(rw(cmd), None, "should not rewrite: {cmd}");
        }
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
