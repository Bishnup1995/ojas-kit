//! Structure scanner for brace languages: Rust, JS/TS, Go, Java, C#.

use regex::Regex;
use std::sync::LazyLock;

use super::{Block, Item, LexLine, SymKind};

static CONTROL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:\}\s*)?(?:else|if|for|foreach|while|do|switch|match|try|catch|finally|loop|lock|fixed|select|defer|go\s+func|using\s*\(|unsafe\s*\{|return|await|yield)\b")
        .unwrap()
});
static IMPORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:pub(?:\([^)]*\))?\s+)?(?:use|import|package|using|extern\s+crate|export\s+\{|export\s+\*|export\s+type\s+\{|from)\b").unwrap()
});
static CONTAINER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:impl|trait|mod|class|interface|namespace|record|object|extern\s*"[^"]*")\b|\btype\s+\w+(?:\[[^\]]*\])?\s+interface\b|\bdeclare\s+module\b|@interface\b"#)
        .unwrap()
});
static DATA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:struct|enum|union)\b|=\s*\{\s*$|\btype\s+\w+.*=\s*\{").unwrap()
});
static DECL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(?:pub(?:\([^)]*\))?|export|declare|default|public|private|protected|internal|static|readonly|final|abstract|const|let|var|async|unsafe|extern|override|virtual|sealed|partial|new)\s+)*(?:const|static|let|var|type|fn|func|function|struct|enum|class|interface|record|delegate|event)\b|^[A-Z_][A-Z0-9_]*\s*[:=]")
        .unwrap()
});
static NAME_KW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:fn|struct|enum|trait|union|mod|type|class|interface|namespace|record|object|func|function|macro_rules!)\s*\*?\s*([A-Za-z_$][\w$]*)").unwrap()
});
static GO_METHOD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^func\s*\([^)]*\)\s*([A-Za-z_]\w*)").unwrap());
static GO_OWNER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^func\s*\(\s*(?:[A-Za-z_]\w*\s+)?\*?\s*([A-Za-z_]\w*)").unwrap());
/// A one-line statement that is really a definition with a body.
static ONE_LINE_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b(?:fn|func|function|class|struct|enum|union|interface|record|trait|impl)\b|=>\s*\{",
    )
    .unwrap()
});
static VAR_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:const|let|var|static)\s+(?:mut\s+)?([A-Za-z_$][\w$]*)").unwrap()
});
static CALL_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([A-Za-z_$][\w$]*)\s*(?:<[^()]*>)?\s*\(").unwrap());
static GENERICS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^<>]*>").unwrap());

const NOT_NAMES: &[&str] = &[
    "if", "for", "while", "switch", "catch", "function", "return", "new", "await", "typeof",
    "sizeof", "async", "super", "this", "base", "match", "fn", "func", "throw", "using", "lock",
];

fn go_owner(header: &str) -> String {
    GO_OWNER
        .captures(header.trim())
        .map(|c| c[1].to_string())
        .unwrap_or_default()
}

pub fn parse(lines: &[LexLine]) -> Vec<Item> {
    parse_region(lines, 0, lines.len(), 0)
}

fn ends_with_any(s: &str, ends: &[&str]) -> bool {
    ends.iter().any(|e| s.ends_with(e))
}

/// Walk back from the line that opens a block to the start of its header
/// (multi-line signatures, attributes, decorators, Allman braces).
fn header_start(lines: &[LexLine], region_start: usize, open: usize, level: i32) -> usize {
    let mut h = open;
    while h > region_start && open - h < 12 {
        let prev = &lines[h - 1];
        if prev.depth_before != level || prev.depth_after != level {
            break;
        }
        let p = prev.code.trim();
        let c = lines[h].code.trim();
        if p.is_empty() {
            break;
        }
        let cont = prev.paren_after > 0
            || ends_with_any(
                p,
                &[",", "(", "[", "=", "=>", "->", ":", "|", "&", "+", "<"],
            )
            || p == "where"
            || p.ends_with(" where")
            || p.starts_with("#[")
            || p.starts_with('@')
            || (p.starts_with('[') && p.ends_with(']'))
            || c.starts_with('{')
            || c.starts_with("where")
            || c.starts_with("->")
            || c.starts_with(':')
            || c.starts_with("extends")
            || c.starts_with("implements")
            || c.starts_with("throws")
            || c.starts_with('.');
        if !cont {
            break;
        }
        h -= 1;
    }
    h
}

/// Header text without leading attribute/decorator/annotation lines.
fn header_text(lines: &[LexLine], h: usize, open: usize) -> String {
    let mut parts = Vec::new();
    for l in &lines[h..=open] {
        let t = l.code.trim();
        let is_attr = t.starts_with("#[")
            || (t.starts_with('@') && !t.starts_with("@interface"))
            || (t.starts_with('[') && t.ends_with(']'));
        if parts.is_empty() && is_attr {
            continue;
        }
        parts.push(t);
    }
    after_last_statement(&parts.join(" ")).to_string()
}

/// `const f = g(); try {` -> `try {`: only the last statement owns the block.
/// Semicolons inside brackets (`for (;;)`, `{ a; b }`) do not count.
fn after_last_statement(s: &str) -> &str {
    let mut depth = 0i32;
    let mut cut = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ';' if depth == 0 => cut = i + 1,
            _ => {}
        }
    }
    s[cut..].trim_start()
}

fn classify(header: &str) -> SymKind {
    if CONTROL.is_match(header) {
        return SymKind::Control;
    }
    if IMPORT.is_match(header) {
        return SymKind::Import;
    }
    // A function header that merely mentions a container word in its params
    // (e.g. `fn f(x: impl Trait) {`) is still a function.
    let before_paren = header.split('(').next().unwrap_or(header);
    if CONTAINER.is_match(before_paren) {
        return SymKind::Container;
    }
    if DATA.is_match(before_paren) || DATA.is_match(header) && !header.contains("=>") {
        return SymKind::Data;
    }
    if header.contains('(') || header.contains("=>") {
        return SymKind::Function;
    }
    SymKind::Other
}

fn extract_name(header: &str) -> String {
    let h = header.trim();
    // `impl` / `unsafe impl` / `default impl` blocks only, not `impl Trait` in params.
    let unprefixed = h
        .trim_start_matches("unsafe ")
        .trim_start_matches("default ");
    if let Some(rest) = unprefixed
        .strip_prefix("impl")
        .filter(|r| r.starts_with([' ', '<']))
    {
        // impl<T> Trait for Type<T> -> Type
        let mut r = rest.to_string();
        while GENERICS.is_match(&r) {
            r = GENERICS.replace_all(&r, "").into_owned();
        }
        let r = r.split('{').next().unwrap_or("");
        let target = r.rsplit_once(" for ").map(|(_, t)| t).unwrap_or(r);
        let ty = target.split_whitespace().next().unwrap_or("");
        return ty.rsplit("::").next().unwrap_or(ty).to_string();
    }
    if let Some(c) = GO_METHOD.captures(h) {
        return c[1].to_string();
    }
    if let Some(c) = NAME_KW.captures(h) {
        return c[1].to_string();
    }
    if let Some(c) = VAR_NAME.captures(h) {
        return c[1].to_string();
    }
    for c in CALL_NAME.captures_iter(h) {
        let n = &c[1];
        if !NOT_NAMES.contains(&n) {
            return n.to_string();
        }
    }
    String::new()
}

fn parse_region(lines: &[LexLine], start: usize, end: usize, level: i32) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    let mut i = start;
    while i < end {
        let l = &lines[i];
        let code = l.code.trim();
        if l.depth_before != level || code.is_empty() {
            i += 1;
            continue;
        }
        if l.depth_after > level {
            // Block opens here; find the line where it closes.
            let mut e = i;
            while e + 1 < end && lines[e].depth_after > level {
                e += 1;
            }
            let h = header_start(lines, start, i, level);
            let header = header_text(lines, h, i);
            let mut kind = classify(&header);
            // At top level a function needs a declaration keyword; a bare call
            // with a callback (`describe("x", () => {`, `app.get(...)`) is not one.
            if level == 0 && kind == SymKind::Function && !DECL.is_match(&header) {
                kind = SymKind::Other;
            }
            let children = if kind == SymKind::Container {
                parse_region(lines, i + 1, e.max(i + 1), level + 1)
            } else {
                Vec::new()
            };
            // Header lines may have been recorded as declarations already.
            items.retain(|it| it.start() < h);
            items.push(Item::Block(Block {
                kind,
                name: extract_name(&header),
                start: h,
                open: i,
                end: e,
                children,
                owner: go_owner(&header),
            }));
            i = e + 1;
            continue;
        }
        // A statement on one line, or spread over lines by open parens.
        let mut j = i;
        // Stop before a line that opens a block: it starts its own header.
        while j + 1 < end && lines[j].paren_after > 0 && lines[j + 1].depth_after <= level {
            j += 1;
        }
        // One-line definition such as `fn a() {}`, `class E {}` or, inside a class,
        // `int getX() { return x; }`. Destructuring like `const { a } = f()` is not.
        let stmt = after_last_statement(code);
        let one_line_def = || {
            let code = stmt;
            let Some(brace) = code.find('{') else {
                return false;
            };
            let method_like = level > 0
                && code
                    .find('(')
                    .is_some_and(|p| p < brace && !code[..p].contains('='));
            (ONE_LINE_DEF.is_match(code) || method_like)
                && matches!(
                    classify(code),
                    SymKind::Function | SymKind::Data | SymKind::Container
                )
        };
        if IMPORT.is_match(code) {
            items.push(Item::Import { start: i, end: j });
        } else if one_line_def() {
            items.push(Item::Block(Block {
                kind: classify(stmt),
                name: extract_name(stmt),
                start: i,
                open: i,
                end: j,
                children: Vec::new(),
                owner: go_owner(stmt),
            }));
        } else if (level > 0 && !CONTROL.is_match(code)) || DECL.is_match(code) {
            items.push(Item::Decl { start: i, end: j });
        }
        i = j + 1;
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::Lang;

    fn names(items: &[Item]) -> Vec<String> {
        items
            .iter()
            .filter_map(|i| match i {
                Item::Block(b) => Some(format!("{:?}:{}", b.kind, b.name)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn rust_structure() {
        let src = r#"use std::io;

#[derive(Debug)]
pub struct Point {
    x: i32,
}

impl<T: Clone> Display for Wrapper<T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", 1)
    }
}

pub fn long_sig(
    a: i32,
    b: impl Trait,
) -> i32
where
    T: Clone,
{
    if a > 0 { return 1; }
    0
}
"#;
        let items = crate::code::parse(src, Lang::Rust);
        assert!(matches!(items[0], Item::Import { start: 0, .. }));
        assert_eq!(
            names(&items),
            vec!["Data:Point", "Container:Wrapper", "Function:long_sig"]
        );
        let Item::Block(p) = &items[1] else { panic!() };
        assert_eq!((p.start, p.open, p.end), (2, 3, 5));
        let Item::Block(im) = &items[2] else { panic!() };
        assert_eq!(names(&im.children), vec!["Function:fmt"]);
        let Item::Block(f) = &items[3] else { panic!() };
        assert_eq!((f.start, f.open), (13, 19));
    }

    #[test]
    fn typescript_class_and_arrows() {
        let src = "import { a } from 'x';\nexport class Foo extends Bar {\n  private n = 1;\n  constructor(x: number) {\n    super();\n  }\n  async run(): Promise<void> {\n    if (x) { y(); }\n  }\n}\nexport const handler = async (req) => {\n  return 1;\n};\nconst config = {\n  a: 1,\n};\n";
        let items = crate::code::parse(src, Lang::TypeScript);
        assert_eq!(
            names(&items),
            vec!["Container:Foo", "Function:handler", "Data:config"]
        );
        let Item::Block(c) = &items[1] else { panic!() };
        assert_eq!(
            names(&c.children),
            vec!["Function:constructor", "Function:run"]
        );
        assert!(matches!(c.children[0], Item::Decl { .. }));
    }

    #[test]
    fn java_allman_and_go() {
        let src = "public class A\n{\n    @Override\n    public String toString()\n    {\n        return \"{\";\n    }\n}\n";
        let items = crate::code::parse(src, Lang::CSharp);
        let Item::Block(c) = &items[0] else { panic!() };
        assert_eq!(c.name, "A");
        let Item::Block(m) = &c.children[0] else {
            panic!()
        };
        assert_eq!(
            (m.name.as_str(), m.start, m.open, m.end),
            ("toString", 2, 4, 6)
        );

        let go = "package main\n\nimport (\n\t\"fmt\"\n)\n\ntype S struct {\n\tA int\n}\n\nfunc (s *S) Do(x int) error {\n\treturn nil\n}\n";
        let items = crate::code::parse(go, Lang::Go);
        assert_eq!(names(&items), vec!["Data:S", "Function:Do"]);
        assert!(matches!(items[1], Item::Import { start: 2, end: 4 }));
    }
}
