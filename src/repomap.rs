//! `tsave map`: an aider-style repo map. Definitions are ranked by how many other
//! files mention their name, then cut to a token budget. The output depends only on
//! the files' contents and is fully sorted, so it is byte-stable across sessions
//! (prompt-cache friendly) until the code changes.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use regex::Regex;
use std::sync::LazyLock;

use crate::code::{self, Block, Item, SymKind};
use crate::filters::common::shorten_line;
use crate::stats::tok;

const MAX_FILES: usize = 5000;
const MAX_FILE_BYTES: u64 = 512 * 1024;
const MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;
const DEFS_PER_FILE: usize = 10;

const SKIP_DIRS: &[&str] = &[
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
    "bin",
    "obj",
];

/// Names too generic to rank on.
const STOP: &[&str] = &[
    "new",
    "main",
    "init",
    "__init__",
    "test",
    "get",
    "set",
    "run",
    "default",
    "from",
    "into",
    "fmt",
    "drop",
    "clone",
    "hash",
    "to_string",
    "constructor",
    "toString",
    "equals",
    "hashCode",
    "String",
    "len",
    "is_empty",
    "next",
    "call",
    "self",
    "this",
    "setup",
    "teardown",
    "setUp",
    "tearDown",
    "handle",
    "update",
    "build",
    "create",
    "close",
    "open",
    "read",
    "write",
    "parse",
];

static IDENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").unwrap());

struct Def {
    name: String,
    line: usize,
    sig: String,
    member: bool,
    /// Types matter more than functions; methods least.
    weight: f64,
}

struct FileInfo {
    path: String,
    defs: Vec<Def>,
    idents: HashSet<String>,
}

fn git_files(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

fn walk(root: &Path, rel: &str, depth: usize, out: &mut Vec<String>) {
    if depth > 8 || out.len() >= MAX_FILES * 4 {
        return;
    }
    let dir = if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_str()) {
                walk(root, &path, depth + 1, out);
            }
        } else if ft.is_file() {
            out.push(path);
        }
    }
}

/// The first non-attribute header line(s) of a block, as one compact line.
fn signature(src: &[&str], b: &Block) -> String {
    let header: Vec<&str> = src
        .iter()
        .take(b.open + 1)
        .skip(b.start)
        .map(|l| l.trim())
        .collect();
    // Python: start at the `def`/`class` line, skipping (multi-line) decorators.
    let py_def = header.iter().position(|t| {
        t.starts_with("def ") || t.starts_with("async def ") || t.starts_with("class ")
    });
    let mut parts: Vec<&str> = Vec::new();
    for t in header.iter().skip(py_def.unwrap_or(0)) {
        let comment = t.starts_with("//") || (t.starts_with('#') && !t.starts_with("#["));
        let attr = t.starts_with("#[")
            || (t.starts_with('@') && !t.starts_with("@interface"))
            || (t.starts_with('[') && t.ends_with(']'));
        if comment || (parts.is_empty() && attr) {
            continue;
        }
        parts.push(t);
    }
    let joined = parts.join(" ");
    let joined = joined.trim_end_matches('{').trim_end();
    shorten_line(joined, 160)
}

fn collect_defs(items: &[Item], src: &[&str], member: bool, out: &mut Vec<Def>) {
    for it in items {
        let Item::Block(b) = it else { continue };
        if matches!(b.kind, SymKind::Control | SymKind::Import | SymKind::Other)
            || b.name.is_empty()
        {
            continue;
        }
        let weight = match (member, b.kind) {
            (true, _) => 0.3,
            // Go methods are top-level but belong to their receiver type.
            _ if !b.owner.is_empty() => 0.3,
            (false, SymKind::Container | SymKind::Data) => 2.0,
            _ => 1.0,
        };
        out.push(Def {
            name: b.name.clone(),
            line: b.start,
            sig: signature(src, b),
            member,
            weight,
        });
        if b.kind == SymKind::Container && !member {
            collect_defs(&b.children, src, true, out);
        }
    }
}

/// Tests, fixtures and benchmarks rarely belong in a map of the codebase.
fn is_test_path(path: &str) -> bool {
    let p = path.replace('\\', "/").to_ascii_lowercase();
    let file = p.rsplit('/').next().unwrap_or(&p);
    p.split('/').any(|seg| {
        matches!(
            seg,
            "test" | "tests" | "__tests__" | "testdata" | "fixtures" | "benches" | "e2e" | "spec"
        )
    }) || file.starts_with("test_")
        || file.contains("_test.")
        || file.contains(".test.")
        || file.contains(".spec.")
        || file.contains("_spec.")
}

fn analyze_file(path: &str, bytes: &[u8], lang: code::Lang) -> FileInfo {
    let src = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = src.split('\n').collect();
    let (items, code_lines) = code::analyze(&src, lang);
    let mut defs = Vec::new();
    collect_defs(&items, &lines, false, &mut defs);
    // One entry per name per file (getter/setter pairs, repeated trait impls).
    let mut seen = HashSet::new();
    defs.retain(|d| seen.insert(d.name.clone()));
    if is_test_path(path) {
        defs.iter_mut().for_each(|d| d.weight *= 0.1);
    }
    let mut idents: HashSet<String> = HashSet::new();
    for l in &code_lines {
        for m in IDENT.find_iter(l) {
            let s = m.as_str();
            if s.len() >= 3 && !idents.contains(s) {
                idents.insert(s.to_string());
            }
        }
    }
    FileInfo {
        path: path.replace('\\', "/"),
        defs,
        idents,
    }
}

/// Code files to scan, with their sizes and modification times (the cache key).
fn candidates(root: &Path) -> Vec<(String, code::Lang, u64, u128)> {
    let mut paths = git_files(root).unwrap_or_else(|| {
        let mut v = Vec::new();
        walk(root, "", 0, &mut v);
        v
    });
    paths.sort();
    paths.dedup();
    let mut out = Vec::new();
    let mut total = 0u64;
    for p in paths {
        if out.len() >= MAX_FILES || total >= MAX_TOTAL_BYTES as u64 {
            break;
        }
        let Some(lang) = code::lang_for(Path::new(&p)) else {
            continue;
        };
        let Ok(meta) = fs::metadata(root.join(&p)) else {
            continue;
        };
        if !meta.is_file() || meta.len() > MAX_FILE_BYTES {
            continue;
        }
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        total += meta.len();
        out.push((p, lang, meta.len(), mtime));
    }
    out
}

/// Build the map for the repository at `root`, cut to about `budget` tokens.
/// The result is cached under `~/.tsave/mapcache`, keyed by every scanned file's
/// path, size and modification time, so unchanged repos answer instantly.
pub fn map(root: &Path, budget: usize) -> String {
    use sha2::{Digest, Sha256};
    let files_meta = candidates(root);
    let mut h = Sha256::new();
    h.update(format!("v1|{budget}|").as_bytes());
    for (p, _, len, mtime) in &files_meta {
        h.update(format!("{p}|{len}|{mtime}\n").as_bytes());
    }
    let key: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    let root_id: String = {
        let abs = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        Sha256::digest(abs.to_string_lossy().as_bytes())
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect()
    };
    let cache = crate::paths::tsave_home()
        .join("mapcache")
        .join(format!("{root_id}.txt"));
    if let Ok(text) = fs::read_to_string(&cache) {
        if let Some(rest) = text.strip_prefix(&format!("{key}\n")) {
            return rest.to_string();
        }
    }
    let result = build(root, &files_meta, budget);
    if fs::create_dir_all(cache.parent().unwrap_or(root)).is_ok() {
        let _ = fs::write(&cache, format!("{key}\n{result}"));
    }
    result
}

fn build(root: &Path, files_meta: &[(String, code::Lang, u64, u128)], budget: usize) -> String {
    // Parse in parallel; keep the sorted path order for determinism.
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(1, 16);
    let chunk = files_meta.len().div_ceil(threads).max(1);
    let files: Vec<FileInfo> = std::thread::scope(|s| {
        let handles: Vec<_> = files_meta
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    part.iter()
                        .filter_map(|(p, lang, _, _)| {
                            let bytes = fs::read(root.join(p)).ok()?;
                            Some(analyze_file(p, &bytes, *lang))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });

    // Document frequency of identifiers, and how many files define each name.
    let mut df: HashMap<&str, usize> = HashMap::new();
    for f in &files {
        for id in &f.idents {
            *df.entry(id.as_str()).or_default() += 1;
        }
    }
    let mut defined_in: HashMap<String, usize> = HashMap::new();
    for f in &files {
        let names: HashSet<&str> = f.defs.iter().map(|d| d.name.as_str()).collect();
        for n in names {
            *defined_in.entry(n.to_string()).or_default() += 1;
        }
    }

    let score = |d: &Def| -> f64 {
        if d.name.len() < 3 || STOP.contains(&d.name.as_str()) {
            return 0.0;
        }
        let refs = df
            .get(d.name.as_str())
            .copied()
            .unwrap_or(0)
            .saturating_sub(1);
        let owners = defined_in.get(&d.name).copied().unwrap_or(1).max(1);
        refs as f64 / owners as f64 * d.weight
    };
    let mut ranked: Vec<(f64, &str, Vec<&Def>)> = Vec::new();
    for f in &files {
        let mut defs: Vec<(f64, &Def)> = f
            .defs
            .iter()
            .map(|d| (score(d), d))
            .filter(|(s, _)| *s > 0.0)
            .collect();
        if defs.is_empty() {
            continue;
        }
        defs.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.line.cmp(&b.1.line)));
        // A file ranks by its best few definitions, so many small getters
        // cannot outweigh one central type.
        let file_score: f64 = defs.iter().take(5).map(|(s, _)| s).sum();
        defs.truncate(DEFS_PER_FILE);
        defs.sort_by_key(|(_, d)| d.line);
        ranked.push((
            file_score,
            f.path.as_str(),
            defs.into_iter().map(|(_, d)| d).collect(),
        ));
    }
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(b.1)));

    let mut out = String::from("Repo map (tsave): most-referenced definitions, by file\n");
    let header_len = out.len();
    for (_, path, defs) in &ranked {
        let mut block = format!("{path}:\n");
        for d in defs {
            let pad = if d.member { "    " } else { "  " };
            block.push_str(&format!("{pad}{}: {}\n", d.line + 1, d.sig));
        }
        if tok(out.len() + block.len()) > budget {
            break;
        }
        out.push_str(&block);
    }
    if out.len() == header_len {
        return String::new();
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_by_cross_file_references() {
        let dir = std::env::temp_dir().join(format!("tsave-map-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/core.rs"), "pub struct Engine {\n    x: i32,\n}\n\npub fn compress_all() {}\n\nfn lonely_helper() {}\n").unwrap();
        fs::write(
            dir.join("src/a.rs"),
            "fn a() { let e = Engine { x: 1 }; compress_all(); }\n",
        )
        .unwrap();
        fs::write(dir.join("src/b.rs"), "fn b() -> Engine { todo!() }\n").unwrap();
        let m = build(&dir, &candidates(&dir), 1500);
        assert!(m.starts_with("Repo map (tsave)"), "{m}");
        let core = m.find("src/core.rs:").expect("core listed");
        assert!(m[core..].contains("1: pub struct Engine"));
        assert!(m[core..].contains("5: pub fn compress_all()"));
        assert!(
            !m.contains("lonely_helper"),
            "unreferenced defs are dropped"
        );
        assert_eq!(m, build(&dir, &candidates(&dir), 1500), "deterministic");
        assert!(
            build(&dir, &candidates(&dir), 10).is_empty(),
            "budget respected"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
