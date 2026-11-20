//! Render an outline (signatures with elided bodies) and look up symbols.

use super::{parse, Block, Item, Lang, SymKind};
use crate::filters::common::shorten_line;

const MAX_DATA_BODY: usize = 15;
const MAX_IMPORTS_SHOWN: usize = 8;

struct Renderer<'a> {
    src: Vec<&'a str>,
    python: bool,
    out: Vec<String>,
}

impl Renderer<'_> {
    fn text(&self, n: usize) -> &str {
        self.src.get(n).map(|s| s.trim()).unwrap_or("")
    }

    fn push(&mut self, n: usize, level: usize, text: &str) {
        let text = shorten_line(text, 300);
        self.out
            .push(format!("{}: {}{}", n + 1, "  ".repeat(level), text));
    }

    fn imports(&mut self, run: &[(usize, usize)], level: usize) {
        let shown = if run.len() <= MAX_IMPORTS_SHOWN {
            run.len()
        } else {
            MAX_IMPORTS_SHOWN - 2
        };
        for &(s, e) in &run[..shown] {
            let mut t = self.text(s).to_string();
            if e > s {
                t.push_str(&format!(" … (+{} lines)", e - s));
            }
            self.push(s, level, &t);
        }
        if shown < run.len() {
            let (first, last) = (run[shown].0, run[run.len() - 1].1);
            self.out.push(format!(
                "{}… +{} more imports (lines {}-{})",
                "  ".repeat(level),
                run.len() - shown,
                first + 1,
                last + 1
            ));
        }
    }

    /// Join a multi-line header (attributes, wrapped parameters) into one line.
    fn header(&self, start: usize, open: usize) -> String {
        let mut s = String::new();
        // Python: decorators shrink to their names (`@click.option`), their
        // argument lines are dropped, and the `def`/`class` line follows.
        let py_def = if self.python {
            (start..=open).find(|&n| {
                let t = self.text(n);
                t.starts_with("def ") || t.starts_with("async def ") || t.starts_with("class ")
            })
        } else {
            None
        };
        for n in start..=open {
            let mut t = self.text(n);
            let comment = t.starts_with("//") || (t.starts_with('#') && !t.starts_with("#["));
            if t.is_empty() || comment {
                continue;
            }
            if py_def.is_some_and(|d| n < d) {
                if !t.starts_with('@') {
                    continue;
                }
                t = t.split('(').next().unwrap_or(t);
            }
            let glue = s.ends_with(['(', '[', '<']) || t.starts_with([')', ']', '>']);
            if t.starts_with([')', ']']) && s.ends_with(',') {
                s.pop();
            }
            if !s.is_empty() && !glue {
                s.push(' ');
            }
            s.push_str(t);
        }
        s
    }

    fn block(&mut self, b: &Block, level: usize) {
        let header = self.header(b.start, b.open);
        let body = b.end.saturating_sub(b.open);
        if body == 0 {
            // One-line definition: show it as is.
            self.push(b.start, level, &header);
            return;
        }
        match b.kind {
            SymKind::Container => {
                self.push(b.start, level, &header);
                self.items(&b.children, level + 1);
            }
            SymKind::Data if body <= MAX_DATA_BODY => {
                self.push(b.start, level, &header);
                for n in b.open + 1..=b.end {
                    let t = self.text(n).to_string();
                    let lvl = if t.starts_with('}') { level } else { level + 1 };
                    self.push(n, lvl, &t);
                }
            }
            _ => {
                let n = if body == 1 {
                    "1 line".to_string()
                } else {
                    format!("{body} lines")
                };
                let marked = if self.python {
                    format!("{header}  … {n}")
                } else if let Some(head) = header.strip_suffix('{') {
                    format!("{head}{{ … {n} }}")
                } else {
                    format!("{header} … {n} }}")
                };
                self.push(b.start, level, &marked);
                // Nested definitions of a huge function, listed inside it.
                self.items(&b.children, level + 1);
            }
        }
    }

    fn items(&mut self, items: &[Item], level: usize) {
        let mut k = 0;
        while k < items.len() {
            let is_import = |it: &Item| match it {
                Item::Import { .. } => true,
                Item::Block(b) => b.kind == SymKind::Import,
                _ => false,
            };
            if is_import(&items[k]) {
                let mut run = Vec::new();
                while k < items.len() && is_import(&items[k]) {
                    match &items[k] {
                        Item::Import { start, end } => run.push((*start, *end)),
                        Item::Block(b) => run.push((b.start, b.end)),
                        _ => {}
                    }
                    k += 1;
                }
                self.imports(&run, level);
                continue;
            }
            match &items[k] {
                Item::Decl { start, end } => {
                    let mut t = self.text(*start).to_string();
                    if end > start {
                        t.push_str(&format!(" … (+{} lines)", end - start));
                    }
                    self.push(*start, level, &t);
                }
                Item::Block(b) if b.kind != SymKind::Control => self.block(b, level),
                _ => {}
            }
            k += 1;
        }
    }
}

/// Outline of a whole file: imports, declarations and signatures, bodies elided.
pub fn outline(src: &str, lang: Lang) -> String {
    let items = parse(src, lang);
    let mut r = Renderer {
        src: src.split('\n').collect(),
        python: lang == Lang::Python,
        out: Vec::new(),
    };
    r.items(&items, 0);
    r.out.join("\n")
}

/// A symbol found by name: qualified name and 0-based inclusive line range.
pub struct Found {
    pub qual: String,
    pub start: usize,
    pub end: usize,
}

fn walk<'a>(items: &'a [Item], parents: &mut Vec<String>, f: &mut dyn FnMut(&'a Block, &[String])) {
    for it in items {
        if let Item::Block(b) = it {
            if b.kind == SymKind::Control || b.kind == SymKind::Import {
                continue;
            }
            if b.owner.is_empty() || !parents.is_empty() {
                f(b, parents);
            } else {
                // Go methods: treat the receiver type as the parent.
                f(b, std::slice::from_ref(&b.owner));
            }
            if !b.children.is_empty() {
                parents.push(b.name.clone());
                walk(&b.children, parents, f);
                parents.pop();
            }
        }
    }
}

fn qualify(parents: &[String], name: &str) -> String {
    let mut q: Vec<&str> = parents
        .iter()
        .map(String::as_str)
        .filter(|p| !p.is_empty())
        .collect();
    q.push(name);
    q.join("::")
}

/// Find symbols by name. `Type::method` and `Class.method` narrow by parent.
pub fn find(src: &str, lang: Lang, query: &str) -> Vec<Found> {
    let parts: Vec<&str> = query
        .split("::")
        .flat_map(|p| p.split('.'))
        .filter(|p| !p.is_empty())
        .collect();
    let Some((target, quals)) = parts.split_last() else {
        return Vec::new();
    };
    let items = parse(src, lang);
    let mut found = Vec::new();
    walk(&items, &mut Vec::new(), &mut |b, parents| {
        let qual_ok =
            quals.is_empty() || parents.last().is_some_and(|p| p == quals[quals.len() - 1]);
        if b.name == *target && qual_ok {
            found.push(Found {
                qual: qualify(parents, &b.name),
                start: b.start,
                end: b.end,
            });
        }
    });
    found
}

/// Qualified names of every symbol, in file order (for "did you mean" lists).
pub fn symbol_names(src: &str, lang: Lang) -> Vec<String> {
    let items = parse(src, lang);
    let mut names = Vec::new();
    walk(&items, &mut Vec::new(), &mut |b, parents| {
        if !b.name.is_empty() {
            names.push(qualify(parents, &b.name));
        }
    });
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST: &str = "use std::fs;\nuse std::io;\n\npub struct P {\n    x: i32,\n}\n\nimpl P {\n    pub fn new() -> Self {\n        let a = 1;\n        let b = 2;\n        P { x: a + b }\n    }\n}\n\nfn helper(\n    a: i32,\n) -> i32 {\n    a\n}\n";

    #[test]
    fn rust_outline() {
        assert_eq!(
            outline(RUST, Lang::Rust),
            "1: use std::fs;\n2: use std::io;\n4: pub struct P {\n5:   x: i32,\n6: }\n8: impl P {\n9:   pub fn new() -> Self { … 4 lines }\n16: fn helper(a: i32) -> i32 { … 2 lines }"
        );
    }

    #[test]
    fn find_symbols() {
        let f = find(RUST, Lang::Rust, "P::new");
        assert_eq!(f.len(), 1);
        assert_eq!(
            (f[0].qual.as_str(), f[0].start, f[0].end),
            ("P::new", 8, 12)
        );
        let both = find(RUST, Lang::Rust, "P");
        assert_eq!(both.len(), 2, "struct P and impl P");
        assert!(find(RUST, Lang::Rust, "Q::new").is_empty());
        assert_eq!(
            symbol_names(RUST, Lang::Rust),
            vec!["P", "P", "P::new", "helper"]
        );
    }

    #[test]
    fn python_outline() {
        let src = "import os\n\nclass A:\n    x: int = 1\n\n    def f(self):\n        return 1\n        # end\n";
        assert_eq!(
            outline(src, Lang::Python),
            "1: import os\n3: class A:\n4:   x: int = 1\n6:   def f(self):  … 1 line"
        );
    }

    #[test]
    fn python_decorators_shrink() {
        let src = "@click.option(\n    \"--port\",\n    default=1,\n)\n# a comment\n@app.get(\"/x\")\ndef serve(port):\n    return port\n";
        assert_eq!(
            outline(src, Lang::Python),
            "1: @click.option @app.get def serve(port):  … 1 line"
        );
    }

    #[test]
    fn huge_functions_list_nested_defs() {
        let mut src = String::from("def create_app(cfg):\n    app = App()\n\n    @app.get(\"/x\")\n    async def handle_x(req):\n        return 1\n");
        for i in 0..210 {
            src.push_str(&format!("    v{i} = {i}\n"));
        }
        src.push_str("    return app\n");
        let o = outline(&src, Lang::Python);
        assert!(o.starts_with("1: def create_app(cfg):  … 216 lines\n4:   @app.get async def handle_x(req):  … 1 line"), "{o}");
        assert_eq!(find(&src, Lang::Python, "create_app::handle_x")[0].start, 3);
    }

    #[test]
    fn go_methods_qualify_by_receiver() {
        let src = "package x\n\ntype Store struct {\n\tpath string\n}\n\nfunc (s *Store) Path() string { return s.path }\n\nfunc (s Store) Open() error {\n\treturn nil\n}\n";
        let f = find(src, Lang::Go, "Store.Path");
        assert_eq!(f.len(), 1);
        assert_eq!(
            (f[0].qual.as_str(), f[0].start, f[0].end),
            ("Store::Path", 6, 6)
        );
        assert_eq!(find(src, Lang::Go, "Store::Open")[0].end, 10);
        assert!(find(src, Lang::Go, "Other.Path").is_empty());
    }
}
