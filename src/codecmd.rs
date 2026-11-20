//! CLI handlers for the code-reading and file-compression commands:
//! `outline`, `read`, `map`, `compress-md`.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::code::{self, outline as ol};
use crate::{config, hook, mdcompress, out, parse_range, paths, repomap, stats};

fn load(file: &Path) -> Result<Vec<u8>, i32> {
    fs::read(file).map_err(|e| {
        eprintln!("tsave: {}: {e}", file.display());
        1
    })
}

fn raw(bytes: &[u8]) {
    let mut o = io::stdout().lock();
    let _ = o.write_all(bytes);
    let _ = o.flush();
}

fn numbered(src: &str, start: usize, end: usize) -> String {
    src.split('\n')
        .enumerate()
        .skip(start)
        .take(end + 1 - start)
        .map(|(i, l)| format!("{}: {}", i + 1, l.trim_end_matches('\r')))
        .collect::<Vec<_>>()
        .join("\n")
}

fn line_count(src: &str) -> usize {
    src.lines().count()
}

pub fn outline(file: &Path) -> i32 {
    let bytes = match load(file) {
        Ok(b) => b,
        Err(c) => return c,
    };
    let Some(lang) = code::lang_for(file) else {
        eprintln!(
            "tsave: no outline support for {} (supported: .rs .py .js .jsx .ts .tsx .go .java .cs)",
            file.display()
        );
        return 2;
    };
    let src = String::from_utf8_lossy(&bytes);
    out(&format!(
        "{} ({} lines, {} outline)\n{}",
        file.display(),
        line_count(&src),
        lang.name(),
        ol::outline(&src, lang)
    ));
    0
}

pub fn read(file: &Path, symbol: Option<&str>, lines: Option<&str>, full: bool) -> i32 {
    let bytes = match load(file) {
        Ok(b) => b,
        Err(c) => return c,
    };
    if full {
        raw(&bytes);
        return 0;
    }
    let src = String::from_utf8_lossy(&bytes);
    let n = line_count(&src);
    if let Some(spec) = lines {
        let Some((s, e)) = parse_range(spec, n) else {
            eprintln!("tsave: invalid range '{spec}' (the file has {n} lines)");
            return 2;
        };
        out(&numbered(&src, s - 1, e - 1));
        return 0;
    }
    let lang = code::lang_for(file);
    if let Some(q) = symbol {
        let Some(lang) = lang else {
            eprintln!(
                "tsave: symbol lookup is not supported for {}",
                file.display()
            );
            return 2;
        };
        let found = ol::find(&src, lang, q);
        if found.is_empty() {
            let names = ol::symbol_names(&src, lang);
            let shown: Vec<&str> = names.iter().take(40).map(String::as_str).collect();
            let more = if names.len() > 40 {
                format!(", … +{}", names.len() - 40)
            } else {
                String::new()
            };
            eprintln!(
                "tsave: no symbol '{q}' in {}. Symbols: {}{more}",
                file.display(),
                shown.join(", ")
            );
            return 1;
        }
        let parts: Vec<String> = found
            .iter()
            .map(|f| {
                format!(
                    "── {} ({}:{}-{})\n{}",
                    f.qual,
                    file.display(),
                    f.start + 1,
                    f.end + 1,
                    numbered(&src, f.start, f.end)
                )
            })
            .collect();
        out(&parts.join("\n"));
        return 0;
    }
    let cfg = config::load();
    match lang {
        Some(lang) if n > cfg.read_min_lines => {
            let body = ol::outline(&src, lang);
            let shown = format!(
                "{} ({n} lines, {} outline; bodies elided)\n{body}\n[tsave: outline of a {n}-line file, ≈{} → ≈{} tokens. Bodies: {bin} read {f} --symbol <name>, or --lines START:END. Whole file: --full]",
                file.display(),
                lang.name(),
                stats::tok(bytes.len()),
                stats::tok(body.len()),
                bin = hook::bin_name(),
                f = file.display(),
            );
            out(&shown);
            stats::record("read", "outline", bytes.len(), shown.len());
        }
        _ => raw(&bytes),
    }
    0
}

pub fn map(root: &Path, budget: usize) -> i32 {
    let m = repomap::map(root, budget);
    if m.is_empty() {
        eprintln!(
            "tsave: no ranked definitions found under {}",
            root.display()
        );
        return 1;
    }
    out(&m);
    0
}

fn backup_name(file: &Path) -> String {
    let abs = fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    let flat: String = abs
        .to_string_lossy()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let tail: String = flat
        .chars()
        .rev()
        .take(100)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}-{tail}.bak", paths::now_secs())
}

pub fn compress_md(file: &Path, dry_run: bool, to_stdout: bool) -> i32 {
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["md", "mdc", "markdown", "txt"].contains(&ext.as_str()) {
        eprintln!("tsave: compress-md only handles .md, .mdc, .markdown and .txt files");
        return 2;
    }
    let src = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("tsave: {}: {e}", file.display());
            return 1;
        }
    };
    let new = mdcompress::compress(&src);
    if let Err(why) = mdcompress::validate(&src, &new) {
        eprintln!(
            "tsave: compression rejected ({why}); {} left unchanged",
            file.display()
        );
        return 1;
    }
    if to_stdout {
        raw(new.as_bytes());
        return 0;
    }
    let (a, b) = (stats::tok(src.len()), stats::tok(new.len()));
    let pct = if a == 0 {
        0.0
    } else {
        100.0 * (a - b) as f64 / a as f64
    };
    let summary = format!("{}: ≈{a} → ≈{b} tokens (-{pct:.0}%)", file.display());
    if dry_run {
        out(&format!("{summary} (dry run, nothing written)"));
        return 0;
    }
    if new == src {
        out(&format!(
            "{}: already compact, nothing to do",
            file.display()
        ));
        return 0;
    }
    let dir = paths::tsave_home().join("backups");
    let backup = dir.join(backup_name(file));
    if let Err(e) = fs::create_dir_all(&dir).and_then(|_| fs::write(&backup, &src)) {
        eprintln!(
            "tsave: could not write backup ({e}); {} left unchanged",
            file.display()
        );
        return 1;
    }
    if let Err(e) = fs::write(file, &new) {
        eprintln!("tsave: could not write {}: {e}", file.display());
        return 1;
    }
    out(&format!("{summary}. Backup: {}", backup.display()));
    0
}
