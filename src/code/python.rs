//! Structure scanner for Python (indentation-based).

use regex::Regex;
use std::sync::LazyLock;

use super::{Block, Item, LexLine, SymKind};

static DEF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:async\s+)?def\s+([A-Za-z_]\w*)").unwrap());
static CLASS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^class\s+([A-Za-z_]\w*)").unwrap());
static IMPORT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:import|from)\s").unwrap());
/// Module-level constants and `__all__`.
static MODULE_DECL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[A-Z_][A-Z0-9_]*|__all__|__version__)\s*(?::[^=]+)?=[^=]").unwrap()
});
/// Class attributes and annotated fields (dataclasses, pydantic).
static CLASS_DECL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_]\w*\s*(?::[^=]+(?:=.*)?|=[^=].*)$").unwrap());

fn indent(raw: &str) -> usize {
    raw.chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

struct Ctx<'a> {
    lines: &'a [LexLine],
    raw: Vec<&'a str>,
}

impl Ctx<'_> {
    fn blank(&self, i: usize) -> bool {
        !self.lines[i].starts_in_string && self.lines[i].code.trim().is_empty()
    }

    /// Last physical line of the logical line starting at `i`.
    fn logical_end(&self, i: usize, end: usize) -> usize {
        let mut j = i;
        while j + 1 < end
            && (self.lines[j].paren_after > 0
                || self.lines[j].code.trim_end().ends_with('\\')
                || self.lines[j + 1].starts_in_string)
        {
            j += 1;
        }
        j
    }

    /// Last line of the indented body after a header ending at `j`.
    fn body_end(&self, j: usize, end: usize, ind: usize) -> usize {
        let mut last = j;
        let mut k = j + 1;
        while k < end {
            if self.blank(k) {
                k += 1;
                continue;
            }
            if self.lines[k].starts_in_string || indent(self.raw[k]) > ind {
                last = k;
                k += 1;
            } else {
                break;
            }
        }
        last
    }

    fn region(&self, start: usize, end: usize, in_class: bool) -> Vec<Item> {
        let mut items = Vec::new();
        let mut decorator_start: Option<usize> = None;
        let mut i = start;
        while i < end {
            if self.blank(i) || self.lines[i].starts_in_string {
                i += 1;
                continue;
            }
            let ind = indent(self.raw[i]);
            let j = self.logical_end(i, end);
            let code = self.lines[i].code.trim();
            if code.starts_with('@') {
                decorator_start.get_or_insert(i);
                i = j + 1;
                continue;
            }
            let def = DEF
                .captures(code)
                .map(|c| (SymKind::Function, c[1].to_string()));
            let class = CLASS
                .captures(code)
                .map(|c| (SymKind::Container, c[1].to_string()));
            if let Some((kind, name)) = def.or(class) {
                let last = self.body_end(j, end, ind);
                let children = if kind == SymKind::Container {
                    self.region(j + 1, last + 1, true)
                } else {
                    Vec::new()
                };
                items.push(Item::Block(Block {
                    kind,
                    name,
                    start: decorator_start.take().unwrap_or(i),
                    open: j,
                    end: last,
                    children,
                    owner: String::new(),
                }));
                i = last + 1;
                continue;
            }
            decorator_start = None;
            if IMPORT.is_match(code) {
                items.push(Item::Import { start: i, end: j });
            } else if (in_class && CLASS_DECL.is_match(code))
                || (!in_class && MODULE_DECL.is_match(code))
            {
                items.push(Item::Decl { start: i, end: j });
            }
            // Skip nested bodies of other compound statements (if/for/with/try).
            i = if code.ends_with(':') {
                self.body_end(j, end, ind) + 1
            } else {
                j + 1
            };
        }
        items
    }
}

pub fn parse(src: &str, lines: &[LexLine]) -> Vec<Item> {
    let ctx = Ctx {
        lines,
        raw: src.split('\n').collect(),
    };
    let n = ctx.lines.len();
    ctx.region(0, n, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_structure() {
        let src = r#"import os
from x import (
    a,
    b,
)

MAX = 10

@dataclass
class Point:
    """Doc with def fake():"""
    x: int = 0
    y: int

    def norm(self,
             p: int) -> float:
        s = """
def not_a_def():
"""
        return 1.0

    @property
    def size(self):
        return 2

if __name__ == "__main__":
    def hidden():
        pass

async def main():
    await x()
"#;
        let items = crate::code::parse(src, crate::code::Lang::Python);
        assert!(matches!(items[0], Item::Import { start: 0, end: 0 }));
        assert!(matches!(items[1], Item::Import { start: 1, end: 4 }));
        assert!(matches!(items[2], Item::Decl { start: 6, .. }));
        let Item::Block(c) = &items[3] else {
            panic!("{:?}", items[3])
        };
        assert_eq!(
            (c.name.as_str(), c.start, c.open, c.end),
            ("Point", 8, 9, 23)
        );
        let kids: Vec<String> = c
            .children
            .iter()
            .filter_map(|i| match i {
                Item::Block(b) => Some(format!("{}:{}-{}-{}", b.name, b.start, b.open, b.end)),
                Item::Decl { start, .. } => Some(format!("decl:{start}")),
                _ => None,
            })
            .collect();
        assert_eq!(
            kids,
            vec!["decl:11", "decl:12", "norm:14-15-19", "size:21-22-23"]
        );
        let Item::Block(m) = &items[4] else { panic!() };
        assert_eq!((m.name.as_str(), m.start, m.end), ("main", 29, 30));
        assert_eq!(items.len(), 5);
    }
}
