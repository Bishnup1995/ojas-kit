//! Lightweight, pure-Rust source structure for outlines, symbol reads and the repo map.
//!
//! This is deliberately not tree-sitter: tree-sitter grammars are C code, and ojas
//! stays free of C dependencies so it builds anywhere with a bare Rust toolchain.
//! Instead a small lexer removes strings and comments, then a brace-aware
//! (Rust, JS/TS, Go, Java, C#) or indent-aware (Python) scanner finds definitions.
//! It is heuristic: good enough to show structure, never used to edit code.

mod brace;
pub mod outline;
mod python;

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
}

impl Lang {
    pub fn name(self) -> &'static str {
        match self {
            Lang::Rust => "rust",
            Lang::Python => "python",
            Lang::JavaScript => "javascript",
            Lang::TypeScript => "typescript",
            Lang::Go => "go",
            Lang::Java => "java",
            Lang::CSharp => "csharp",
        }
    }
}

/// Function bodies longer than this (in lines) also list their nested definitions.
pub(crate) const LARGE_BODY: usize = 200;

pub fn lang_for(path: &Path) -> Option<Lang> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" => Lang::Rust,
        "py" | "pyi" => Lang::Python,
        "js" | "jsx" | "mjs" | "cjs" => Lang::JavaScript,
        "ts" | "tsx" | "mts" | "cts" => Lang::TypeScript,
        "go" => Lang::Go,
        "java" => Lang::Java,
        "cs" => Lang::CSharp,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymKind {
    /// impl, trait, mod, class, interface, namespace: members are shown.
    Container,
    /// Function or method: body is elided.
    Function,
    /// struct, enum, object literal: short bodies are shown.
    Data,
    /// Multi-line import (`use a::{...}`, `import {...} from`).
    Import,
    /// Control flow at member level (if/for/try...): hidden.
    Control,
    /// Anything else with a block (macro calls, test `describe(...)`): elided.
    Other,
}

/// A definition with a body. Line numbers are 0-based.
#[derive(Debug, Clone)]
pub struct Block {
    pub kind: SymKind,
    pub name: String,
    /// First header line (includes attributes, decorators, annotations).
    pub start: usize,
    /// Last header line: where the body opens.
    pub open: usize,
    /// Last line of the body.
    pub end: usize,
    pub children: Vec<Item>,
    /// Receiver type for Go methods (`func (s *Store) Path()` -> `Store`), else empty.
    pub owner: String,
}

#[derive(Debug, Clone)]
pub enum Item {
    Import { start: usize, end: usize },
    Decl { start: usize, end: usize },
    Block(Block),
}

impl Item {
    pub fn start(&self) -> usize {
        match self {
            Item::Import { start, .. } | Item::Decl { start, .. } => *start,
            Item::Block(b) => b.start,
        }
    }
}

/// Parse a source file into its top-level items.
pub fn parse(src: &str, lang: Lang) -> Vec<Item> {
    parse_lexed(src, &lex(src, lang), lang)
}

fn parse_lexed(src: &str, lines: &[LexLine], lang: Lang) -> Vec<Item> {
    match lang {
        Lang::Python => python::parse(src, lines),
        _ => brace::parse(lines),
    }
}

/// Items plus the code-only text of each line (comments removed, string contents
/// emptied), lexing the file once. The repo map uses the code lines to find
/// identifier references.
pub fn analyze(src: &str, lang: Lang) -> (Vec<Item>, Vec<String>) {
    let lines = lex(src, lang);
    let items = parse_lexed(src, &lines, lang);
    (items, lines.into_iter().map(|l| l.code).collect())
}

/// One lexed line.
#[derive(Debug, Clone, Default)]
pub(crate) struct LexLine {
    /// Code with comments removed and string contents emptied (`""`).
    pub code: String,
    /// Brace depth before and after this line.
    pub depth_before: i32,
    pub depth_after: i32,
    /// Paren + bracket depth after this line.
    pub paren_after: i32,
    /// True when the line starts inside a multi-line string.
    pub starts_in_string: bool,
}

#[derive(Clone)]
enum State {
    Normal,
    Block,
    Str {
        close: String,
        esc: bool,
        multiline: bool,
    },
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

fn starts_at(chars: &[char], i: usize, s: &str) -> bool {
    s.chars()
        .enumerate()
        .all(|(k, c)| chars.get(i + k) == Some(&c))
}

/// Lex the whole file into per-line code with brace and paren depths.
pub(crate) fn lex(src: &str, lang: Lang) -> Vec<LexLine> {
    let line_comment = if lang == Lang::Python { "#" } else { "//" };
    let mut out = Vec::new();
    let mut st = State::Normal;
    let (mut depth, mut paren) = (0i32, 0i32);
    let mut prev_line_sig: Option<char> = None;
    // Brace depth at each open JS template `${`, innermost last.
    let mut templates: Vec<i32> = Vec::new();
    let is_js = matches!(lang, Lang::JavaScript | Lang::TypeScript);
    for raw in src.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let chars: Vec<char> = raw.chars().collect();
        let starts_in_string = matches!(st, State::Str { .. });
        let depth_before = depth;
        let mut code = String::with_capacity(chars.len());
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            match &st {
                State::Block => {
                    if starts_at(&chars, i, "*/") {
                        st = State::Normal;
                        i += 2;
                    } else {
                        i += 1;
                    }
                    continue;
                }
                State::Str { close, esc, .. } => {
                    if *esc && c == '\\' {
                        i += 2;
                        continue;
                    }
                    if starts_at(&chars, i, close) {
                        i += close.chars().count();
                        code.push('"');
                        st = State::Normal;
                    } else if close == "`" && is_js && starts_at(&chars, i, "${") {
                        // Template expression: lex as code until the matching `}`.
                        templates.push(depth);
                        st = State::Normal;
                        i += 2;
                    } else {
                        i += 1;
                    }
                    continue;
                }
                State::Normal => {}
            }
            if starts_at(&chars, i, line_comment) {
                break;
            }
            if lang != Lang::Python && starts_at(&chars, i, "/*") {
                st = State::Block;
                i += 2;
                continue;
            }
            if c == '/'
                && matches!(lang, Lang::JavaScript | Lang::TypeScript)
                && regex_allowed(&code, prev_line_sig)
            {
                if let Some(end) = js_regex_end(&chars, i) {
                    code.push_str("\"\"");
                    i = end;
                    continue;
                }
            }
            let prev_ident = i > 0 && is_ident(chars[i - 1]);
            if let Some((skip, new_st)) = string_open(&chars, i, lang, prev_ident) {
                code.push('"');
                i += skip;
                if let Some(s) = new_st {
                    st = s;
                } else {
                    code.push('"'); // char literal fully consumed
                }
                continue;
            }
            if c == '}' && templates.last() == Some(&depth) {
                // End of a `${...}` expression: back into the template string.
                templates.pop();
                st = State::Str {
                    close: "`".into(),
                    esc: true,
                    multiline: true,
                };
                i += 1;
                continue;
            }
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                '(' | '[' => paren += 1,
                ')' | ']' => paren = (paren - 1).max(0),
                _ => {}
            }
            code.push(c);
            i += 1;
        }
        if let Some(last) = code.trim_end().chars().last() {
            prev_line_sig = Some(last);
        }
        // Single-line strings cannot continue; recover from unterminated ones.
        if let State::Str {
            multiline: false, ..
        } = st
        {
            st = State::Normal;
        }
        out.push(LexLine {
            code,
            depth_before,
            depth_after: depth,
            paren_after: paren,
            starts_in_string,
        });
    }
    out
}

/// In JS/TS, may a `/` here start a regex literal (rather than divide)? True after
/// an operator, an opening bracket, a comma, or a keyword like `return`.
fn regex_allowed(code_so_far: &str, prev_line_sig: Option<char>) -> bool {
    let t = code_so_far.trim_end();
    let Some(last) = t.chars().last().or(prev_line_sig) else {
        return true;
    };
    if t.is_empty() {
        // Line start: a regex unless the previous line ended an expression.
        return !(is_ident(last) || last == ')' || last == ']');
    }
    if is_ident(last) {
        let word: String = t
            .chars()
            .rev()
            .take_while(|c| is_ident(*c))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return matches!(
            word.as_str(),
            "return"
                | "typeof"
                | "case"
                | "in"
                | "of"
                | "delete"
                | "void"
                | "throw"
                | "new"
                | "yield"
                | "await"
                | "else"
                | "do"
        );
    }
    "(,=:[!&|?{};+-*%<>~^".contains(last)
}

/// End index (exclusive, flags included) of a regex literal starting at `i`, or
/// `None` if the line has no closing `/`.
fn js_regex_end(chars: &[char], i: usize) -> Option<usize> {
    let mut j = i + 1;
    let mut in_class = false;
    while j < chars.len() {
        match chars[j] {
            '\\' => j += 1,
            '[' => in_class = true,
            ']' => in_class = false,
            '/' if !in_class => {
                if j == i + 1 {
                    return None; // `//` is a comment, handled elsewhere
                }
                j += 1;
                while j < chars.len() && chars[j].is_ascii_alphabetic() {
                    j += 1;
                }
                return Some(j);
            }
            _ => {}
        }
        j += 1;
    }
    None
}

/// Detect a string or char literal opening at `i`. Returns how many chars to skip
/// and the new state (`None` means a char literal was consumed whole).
fn string_open(
    chars: &[char],
    i: usize,
    lang: Lang,
    prev_ident: bool,
) -> Option<(usize, Option<State>)> {
    let c = chars[i];
    let s = |close: &str, esc: bool, multiline: bool| State::Str {
        close: close.to_string(),
        esc,
        multiline,
    };
    match lang {
        Lang::Python => {
            // Optional prefix (r, b, f, u, rb, br, fr, rf) then a quote.
            let mut j = i;
            if !prev_ident {
                while j < chars.len() && j - i < 2 && "rRbBfFuU".contains(chars[j]) {
                    j += 1;
                }
            }
            let q = *chars.get(j)?;
            if q != '"' && q != '\'' {
                return None;
            }
            if j > i && prev_ident {
                return None;
            }
            let raw = chars[i..j].iter().any(|c| *c == 'r' || *c == 'R');
            let triple: String = std::iter::repeat_n(q, 3).collect();
            if starts_at(chars, j, &triple) {
                return Some((j - i + 3, Some(s(&triple, !raw, true))));
            }
            Some((j - i + 1, Some(s(&q.to_string(), !raw, false))))
        }
        Lang::Rust => {
            // Raw strings: r"..", r#".."#, br#".."#
            if !prev_ident && (c == 'r' || (c == 'b' && chars.get(i + 1) == Some(&'r'))) {
                let mut j = if c == 'b' { i + 2 } else { i + 1 };
                let mut hashes = 0;
                while chars.get(j) == Some(&'#') {
                    hashes += 1;
                    j += 1;
                }
                if chars.get(j) == Some(&'"') {
                    let close = format!("\"{}", "#".repeat(hashes));
                    return Some((j - i + 1, Some(s(&close, false, true))));
                }
            }
            if c == '"' {
                return Some((1, Some(s("\"", true, true))));
            }
            char_literal(chars, i)
        }
        Lang::JavaScript | Lang::TypeScript => match c {
            '"' | '\'' => Some((1, Some(s(&c.to_string(), true, false)))),
            '`' => Some((1, Some(s("`", true, true)))),
            _ => None,
        },
        Lang::Go => match c {
            '"' => Some((1, Some(s("\"", true, false)))),
            '`' => Some((1, Some(s("`", false, true)))),
            '\'' => char_literal(chars, i),
            _ => None,
        },
        Lang::Java | Lang::CSharp => {
            if starts_at(chars, i, "\"\"\"") {
                return Some((3, Some(s("\"\"\"", false, true))));
            }
            if lang == Lang::CSharp
                && (starts_at(chars, i, "@\"")
                    || starts_at(chars, i, "$@\"")
                    || starts_at(chars, i, "@$\""))
            {
                let skip = if c == '@' && chars[i + 1] == '"' {
                    2
                } else {
                    3
                };
                return Some((skip, Some(s("\"", false, true))));
            }
            match c {
                '"' => Some((1, Some(s("\"", true, false)))),
                '\'' => char_literal(chars, i),
                _ => None,
            }
        }
    }
}

/// `'x'`, `'\n'`, `'\u{1F600}'`. A lone `'` (Rust lifetime) is not a literal.
fn char_literal(chars: &[char], i: usize) -> Option<(usize, Option<State>)> {
    if chars[i] != '\'' {
        return None;
    }
    if chars.get(i + 1) == Some(&'\\') {
        let mut j = i + 2;
        while j < chars.len() && j - i < 12 {
            if chars[j] == '\'' {
                return Some((j - i + 1, None));
            }
            j += 1;
        }
        return None;
    }
    if chars.get(i + 2) == Some(&'\'') {
        return Some((3, None));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_ignores_braces_in_strings_and_comments() {
        let src = "fn a() { let s = \"{\"; // }\n  /* { */ let c = '{'; }\nfn b<'a>(x: &'a str) {}";
        let l = lex(src, Lang::Rust);
        assert_eq!(l[0].depth_after, 1);
        assert_eq!(l[1].depth_after, 0);
        assert_eq!(l[2].depth_after, 0);
        assert_eq!(l[0].code, "fn a() { let s = \"\"; ");
    }

    #[test]
    fn multiline_strings_tracked() {
        let src = "x = \"\"\"\n{ not code\n\"\"\"\ny = 1";
        let l = lex(src, Lang::Python);
        assert!(l[1].starts_in_string);
        assert!(l[2].starts_in_string);
        assert!(!l[3].starts_in_string);
        let js = "const t = `a\n{ b }\n`; function f() {}";
        let l = lex(js, Lang::JavaScript);
        assert_eq!(l[1].depth_after, 0);
        assert_eq!(l[2].depth_after, 0);
    }

    #[test]
    fn js_regex_literals_do_not_break_depth() {
        let js = "const q = `'${s.replace(/'/g, `'\\\\''${`{`}`)}'`;\nconst b = /\\{+/.test(x);\nfunction f() {\n  return a / b / c;\n}\nif (ok) /x{2}/.exec(y)\n";
        let l = lex(js, Lang::JavaScript);
        assert_eq!(l[0].depth_after, 0, "{:?}", l[0].code);
        assert_eq!(l[1].depth_after, 0, "{:?}", l[1].code);
        assert_eq!(l[2].depth_after, 1);
        assert_eq!(l[3].code.trim(), "return a / b / c;");
        assert_eq!(l[4].depth_after, 0);
        assert_eq!(l[5].depth_after, 0);
    }

    #[test]
    fn rust_raw_strings() {
        let src = "let s = r#\"{ \"quoted\" }\"#; fn x() {";
        assert_eq!(lex(src, Lang::Rust)[0].depth_after, 1);
    }
}
