//! `tsave compress-md`: rule-based compression of always-loaded instruction files
//! (CLAUDE.md, AGENTS.md, ...). No LLM: fixed phrase rewrites and filler removal on
//! prose only. Code blocks, inline code, URLs, paths, headings, tables, front matter
//! and list structure are never changed, and a validator checks that before the
//! result is accepted.

use regex::Regex;
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// Protected spans inside a prose line.
static PROTECTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"`[^`]*`|!?\[[^\]]*\]\([^)]*\)|<https?://[^>]+>|https?://\S+|\S*[/\\]\S*|\S+\.[A-Za-z0-9]{1,6}\b|\*\*|__").unwrap()
});
static LIST_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*(?:>\s*)*(?:[-*+]\s+(?:\[[ xX]\]\s+)?|\d+[.)]\s+)?)(.*)$").unwrap()
});
static BULLET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:>\s*)*(?:[-*+]|\d+[.)])\s").unwrap());
static URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"https?://[^\s)>\]]+").unwrap());
static INLINE_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`[^`\n]+`").unwrap());
static FILLER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:just|really|basically|actually|simply|very|quite|essentially|literally|definitely|certainly|obviously|kindly|please)\b,?\s*").unwrap()
});
static HEDGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:I think|I believe|it seems that|in my opinion,?|it should be noted that|it is worth noting that)\s*").unwrap()
});
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" {2,}").unwrap());
static SPACE_PUNCT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" ([,.;:!?])").unwrap());

const PHRASES: &[(&str, &str)] = &[
    ("due to the fact that", "because"),
    ("in order to", "to"),
    ("so as to", "to"),
    ("in the event that", "if"),
    ("at this point in time", "now"),
    ("at the moment", "now"),
    ("for the purpose of", "for"),
    ("with regard to", "about"),
    ("with respect to", "about"),
    ("is able to", "can"),
    ("are able to", "can"),
    ("make sure that", "ensure"),
    ("make sure", "ensure"),
    ("please note that", "note:"),
    ("a lot of", "many"),
    ("as well as", "and"),
    ("prior to", "before"),
    ("whether or not", "whether"),
    ("each and every", "every"),
    ("the majority of", "most"),
    ("on a regular basis", "regularly"),
    ("in addition to", "besides"),
    ("is going to", "will"),
    ("are going to", "will"),
];

static PHRASE_RES: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    PHRASES
        .iter()
        .map(|(from, to)| {
            (
                Regex::new(&format!(r"(?i)\b{}\b", regex::escape(from))).unwrap(),
                *to,
            )
        })
        .collect()
});

fn match_case(original: &str, replacement: &str) -> String {
    let upper = original.chars().next().is_some_and(char::is_uppercase);
    if upper {
        let mut c = replacement.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    } else {
        replacement.to_string()
    }
}

fn compress_free(text: &str) -> String {
    let mut s = text.to_string();
    for (re, to) in PHRASE_RES.iter() {
        s = re
            .replace_all(&s, |c: &regex::Captures| match_case(&c[0], to))
            .into_owned();
    }
    s = HEDGE.replace_all(&s, "").into_owned();
    s = FILLER.replace_all(&s, "").into_owned();
    let s = SPACES.replace_all(&s, " ");
    SPACE_PUNCT.replace_all(&s, "$1").into_owned()
}

/// Compress one prose fragment, leaving protected spans byte-for-byte intact.
fn compress_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for m in PROTECTED.find_iter(text) {
        out.push_str(&compress_free(&text[last..m.start()]));
        out.push_str(m.as_str());
        last = m.end();
    }
    out.push_str(&compress_free(&text[last..]));
    let mut out = out.trim().to_string();
    // Keep a capital at the start if the original had one.
    if text
        .trim_start()
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
    {
        if let Some(f) = out.chars().next().filter(|c| c.is_lowercase()) {
            out = f.to_uppercase().collect::<String>() + &out[f.len_utf8()..];
        }
    }
    out
}

#[derive(PartialEq, Clone, Copy)]
enum LineKind {
    Fence,
    Code,
    Heading,
    Table,
    Front,
    Blank,
    Prose,
}

/// Classify each line once so compression and validation agree.
fn classify(src: &str) -> Vec<(LineKind, &str)> {
    let mut out = Vec::new();
    let mut fence: Option<&str> = None;
    let mut front = false;
    for (i, line) in src.lines().enumerate() {
        let t = line.trim_start();
        let kind = if i == 0 && line.trim() == "---" {
            front = true;
            LineKind::Front
        } else if front {
            if line.trim() == "---" {
                front = false;
            }
            LineKind::Front
        } else if let Some(f) = fence {
            if t.starts_with(f) {
                fence = None;
            }
            LineKind::Fence
        } else if t.starts_with("```") || t.starts_with("~~~") {
            fence = Some(&t[..3]);
            LineKind::Fence
        } else if line.trim().is_empty() {
            LineKind::Blank
        } else if t.starts_with('#') {
            LineKind::Heading
        } else if t.starts_with('|') {
            LineKind::Table
        } else if line.starts_with("    ") || line.starts_with('\t') || t.starts_with('<') {
            LineKind::Code
        } else {
            LineKind::Prose
        };
        out.push((kind, line));
    }
    out
}

pub fn compress(src: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for (kind, line) in classify(src) {
        match kind {
            LineKind::Blank => {
                if out.last().is_some_and(|l| !l.is_empty()) {
                    out.push(String::new());
                }
            }
            LineKind::Prose => {
                let c = LIST_PREFIX.captures(line).unwrap();
                let (prefix, rest) = (&c[1], &c[2]);
                let body = compress_text(rest);
                out.push(format!("{prefix}{body}").trim_end().to_string());
            }
            LineKind::Fence | LineKind::Front => out.push(line.to_string()),
            LineKind::Table => out.push(compact_table_row(line)),
            _ => out.push(line.trim_end().to_string()),
        }
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    // Keep the file's line-ending style.
    let eol = if src.contains("\r\n") { "\r\n" } else { "\n" };
    let mut s = out.join(eol);
    s.push_str(eol);
    s
}

fn multiset<'a>(it: impl Iterator<Item = &'a str>) -> BTreeMap<&'a str, usize> {
    let mut m = BTreeMap::new();
    for s in it {
        *m.entry(s).or_default() += 1;
    }
    m
}

/// Split a table row into trimmed cells (pipes inside backticks don't split), with
/// separator cells normalized to `---` keeping alignment colons.
fn table_cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut in_code = false;
    let mut prev = '\0';
    for c in t.chars() {
        match c {
            '`' => in_code = !in_code,
            '|' if !in_code && prev != '\\' => {
                cells.push(std::mem::take(&mut cur));
                prev = c;
                continue;
            }
            _ => {}
        }
        cur.push(c);
        prev = c;
    }
    cells.push(cur);
    cells
        .into_iter()
        .map(|c| {
            let c = c.trim();
            if !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':') && c.contains('-') {
                let left = if c.starts_with(':') { ":" } else { "" };
                let right = if c.ends_with(':') && c.len() > 1 {
                    ":"
                } else {
                    ""
                };
                format!("{left}---{right}")
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// `| a      | b    |` -> `| a | b |`; `|------|:----:|` -> `|---|:---:|`.
fn compact_table_row(line: &str) -> String {
    let cells = table_cells(line);
    let indent = &line[..line.len() - line.trim_start().len()];
    if cells.iter().all(|c| c.trim_matches(':') == "---") {
        format!("{indent}|{}|", cells.join("|"))
    } else {
        format!("{indent}| {} |", cells.join(" | "))
    }
}

/// Check that nothing structural changed. Returns a description of the first problem.
pub fn validate(orig: &str, new: &str) -> Result<(), String> {
    let (a, b) = (classify(orig), classify(new));
    // Tables compare by cell contents, since padding is allowed to change.
    let pick = |v: &[(LineKind, &str)], k: LineKind| -> Vec<String> {
        v.iter()
            .filter(|(kk, _)| *kk == k)
            .map(|(_, l)| {
                if k == LineKind::Table {
                    table_cells(l).join("|")
                } else {
                    l.trim_end().to_string()
                }
            })
            .collect()
    };
    for (k, what) in [
        (LineKind::Heading, "headings"),
        (LineKind::Fence, "code blocks"),
        (LineKind::Table, "tables"),
        (LineKind::Front, "front matter"),
        (LineKind::Code, "indented code/HTML"),
    ] {
        if pick(&a, k) != pick(&b, k) {
            return Err(format!("{what} changed"));
        }
    }
    let prose = |v: &[(LineKind, &str)]| -> String {
        v.iter()
            .filter(|(k, _)| *k == LineKind::Prose)
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join("\n")
    };
    let (pa, pb) = (prose(&a), prose(&b));
    if multiset(URL.find_iter(&pa).map(|m| m.as_str()))
        != multiset(URL.find_iter(&pb).map(|m| m.as_str()))
    {
        return Err("URLs changed".into());
    }
    if multiset(INLINE_CODE.find_iter(&pa).map(|m| m.as_str()))
        != multiset(INLINE_CODE.find_iter(&pb).map(|m| m.as_str()))
    {
        return Err("inline code changed".into());
    }
    let bullets = |v: &[(LineKind, &str)]| {
        v.iter()
            .filter(|(k, l)| *k == LineKind::Prose && BULLET.is_match(l))
            .count()
    };
    if bullets(&a) != bullets(&b) {
        return Err("list items changed".into());
    }
    if new.len() > orig.len() {
        return Err("result is longer than the original".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compresses_prose_only() {
        let src = "---\ntitle: x\n---\n# Rules\n\n\n\nIn order to build, you should basically run `make  build` really fast.\n- Please make sure that tests pass in src/very/just.rs\n- It is not okay to skip tests.\n\n```sh\njust   really   keep this\n```\n| a | very |\n    indented just code\nSee https://example.com/very/just for more.\n";
        let out = compress(src);
        assert_eq!(
            out,
            "---\ntitle: x\n---\n# Rules\n\nTo build, you should run `make  build` fast.\n- Ensure tests pass in src/very/just.rs\n- It is not okay to skip tests.\n\n```sh\njust   really   keep this\n```\n| a | very |\n    indented just code\nSee https://example.com/very/just for more.\n"
        );
        validate(src, &out).unwrap();
    }

    #[test]
    fn compacts_padded_tables() {
        let src = "| Name      | Value `a | b`  |\n|-----------|:-------------:|\n| x         | 1             |\n";
        let out = compress(src);
        assert_eq!(out, "| Name | Value `a | b` |\n|---|:---:|\n| x | 1 |\n");
        validate(src, &out).unwrap();
        assert!(validate(src, "| Name | Value |\n|---|:---:|\n| x | 1 |\n").is_err());
    }

    #[test]
    fn keeps_crlf_line_endings() {
        let src = "# T\r\n\r\nIn order to go, run `x`.\r\n";
        assert_eq!(compress(src), "# T\r\n\r\nTo go, run `x`.\r\n");
    }

    #[test]
    fn validator_catches_damage() {
        let src = "# H\n- a `x` https://a.b/c\n- b\n";
        assert!(validate(src, "# H2\n- a `x` https://a.b/c\n- b\n")
            .unwrap_err()
            .contains("headings"));
        assert!(validate(src, "# H\n- a https://a.b/c\n- b\n")
            .unwrap_err()
            .contains("inline code"));
        assert!(validate(src, "# H\n- a `x` https://a.b/d\n- b\n")
            .unwrap_err()
            .contains("URLs"));
        assert!(validate(src, "# H\n- a `x` https://a.b/c\n")
            .unwrap_err()
            .contains("list items"));
    }
}
