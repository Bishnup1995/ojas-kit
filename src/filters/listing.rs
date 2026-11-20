//! find / fd / ls output: group paths by directory and hide dependency and build folders.

use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

static LS_LONG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([\-dlcbps])[rwxsStT\-]{9}[@+.]?\s+\d+\s+\S+\s+\S+\s+(\S+)\s+(?:\w{3}\s+\d+\s+[\d:]+|\d{4}-\d\d-\d\d\s+[\d:]+)\s+(.+)$",
    )
    .unwrap()
});

const NOISE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    "venv",
    ".next",
    ".nuxt",
    ".cache",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".gradle",
    "vendor",
    "coverage",
];

const NAMES_PER_DIR: usize = 12;

pub fn compress(text: &str) -> String {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return text.to_string();
    }
    let long = lines.iter().filter(|l| LS_LONG.is_match(l)).count();
    if long * 2 > lines.len() {
        return ls_long(&lines);
    }
    let pathy = lines
        .iter()
        .filter(|l| !l.starts_with(' ') && (l.contains('/') || l.contains('\\')))
        .count();
    if pathy * 10 >= lines.len() * 8 {
        return paths(&lines);
    }
    text.to_string()
}

/// `ls -l`: keep just name and size.
fn ls_long(lines: &[&str]) -> String {
    let mut out = Vec::new();
    for l in lines {
        if l.starts_with("total ") {
            continue;
        }
        match LS_LONG.captures(l) {
            Some(c) if &c[1] == "d" => out.push(format!("{}/", &c[3])),
            Some(c) => out.push(format!("{}  {}", &c[3], &c[2])),
            None => out.push(l.to_string()),
        }
    }
    out.join("\n")
}

fn noise_root(p: &str) -> Option<String> {
    let parts: Vec<&str> = p.split('/').collect();
    let i = parts.iter().position(|c| NOISE_DIRS.contains(c))?;
    (i + 1 < parts.len()).then(|| parts[..=i].join("/"))
}

/// One line per directory: `src/filters/ (7): a.rs, b.rs, …`.
fn paths(lines: &[&str]) -> String {
    let mut order: Vec<String> = Vec::new();
    let mut by_dir: HashMap<String, Vec<String>> = HashMap::new();
    let mut noise: BTreeMap<String, usize> = BTreeMap::new();
    for l in lines {
        let p = l.trim().replace('\\', "/");
        let p = p.strip_prefix("./").unwrap_or(&p).trim_end_matches('/');
        if p.is_empty() || p == "." {
            continue;
        }
        if let Some(root) = noise_root(p) {
            *noise.entry(root).or_default() += 1;
            continue;
        }
        let (dir, name) = match p.rfind('/') {
            Some(i) => (&p[..i], &p[i + 1..]),
            None => (".", p),
        };
        by_dir
            .entry(dir.to_string())
            .or_insert_with(|| {
                order.push(dir.to_string());
                Vec::new()
            })
            .push(name.to_string());
    }
    let total: usize = by_dir.values().map(Vec::len).sum();
    let mut out = vec![format!("{total} paths in {} directories", order.len())];
    for dir in &order {
        let names = &by_dir[dir];
        if names.len() <= NAMES_PER_DIR {
            out.push(format!("{dir}/: {}", names.join(", ")));
        } else {
            out.push(format!(
                "{dir}/ ({}): {}, … +{} more",
                names.len(),
                names[..NAMES_PER_DIR - 2].join(", "),
                names.len() - (NAMES_PER_DIR - 2)
            ));
        }
    }
    for (root, count) in noise {
        out.push(format!("{root}/ … {count} entries hidden"));
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_output_grouped() {
        let raw = "./src/main.rs\n./src/lib.rs\n./node_modules/a/index.js\n./node_modules/b/index.js\n./README.md\n";
        let out = compress(raw);
        assert_eq!(
            out,
            "3 paths in 2 directories\nsrc/: main.rs, lib.rs\n./: README.md\nnode_modules/ … 2 entries hidden"
        );
    }

    #[test]
    fn ls_long_trimmed() {
        let raw = "total 8\ndrwxr-xr-x 2 me staff 4096 Nov 19 10:00 src\n-rw-r--r-- 1 me staff 1234 Nov 19 10:00 Cargo.toml\n";
        assert_eq!(compress(raw), "src/\nCargo.toml  1234");
    }
}
