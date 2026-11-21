//! Claude Code `PostToolUse` hook: compress a tool's output *after* it ran.
//!
//! The PreToolUse rewrite can only help plain simple commands. This hook covers
//! everything else (pipes, loops, compound commands, `cd x && …`) and the built-in
//! Grep tool, because it never changes what runs: Claude Code has already executed
//! the tool, and the hook only replaces what the agent sees, via
//! `hookSpecificOutput.updatedToolOutput`. The original is stored first, so nothing
//! is lost (`ojas get <id>`). Like every handler it fails open: on any problem it
//! prints nothing and the agent sees the normal output.
//!
//! For output over Claude Code's inline limit (about 30k characters) the hook gets
//! only the first 30k in `stdout`, plus `persistedOutputPath` holding the full
//! text. We read that file so the filter sees all of it.

use serde_json::{json, Value};
use std::io::Read;
use std::path::Path;

use crate::config::Config;
use crate::filters::{self, Kind};
use crate::{code, config, stats, store};

/// Never read a persisted output file bigger than this (Claude Code truncates
/// them at 64 MiB anyway).
const MAX_PERSISTED: u64 = 64 * 1024 * 1024;

pub fn post_tool_use() {
    if std::env::var_os("OJAS_DISABLE").is_some() {
        return;
    }
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(&input) else {
        return;
    };
    let cfg = config::load();
    if !cfg.enabled {
        return;
    }
    if let Some(out) = response(&v, &cfg) {
        println!("{out}");
    }
}

/// The hook's JSON answer, or `None` to leave the tool output alone.
pub fn response(v: &Value, cfg: &Config) -> Option<Value> {
    let tool = v.get("tool_name")?.as_str()?;
    let input = v.get("tool_input")?;
    let resp = v.get("tool_response")?.as_object()?;
    let updated = match tool {
        "Bash" => bash(input, resp, cfg)?,
        "Grep" => grep(resp, cfg)?,
        "Read" => read(input, resp, cfg)?,
        _ => return None,
    };
    Some(json!({
        "hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "updatedToolOutput": updated
        }
    }))
}

fn bash(input: &Value, resp: &serde_json::Map<String, Value>, cfg: &Config) -> Option<Value> {
    if input.get("run_in_background").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let cmd = input.get("command")?.as_str()?;
    if already_handled(cmd) {
        return None;
    }
    let full = match resp.get("persistedOutputPath").and_then(Value::as_str) {
        Some(p) => read_persisted(p)?,
        None => resp.get("stdout")?.as_str()?.to_string(),
    };
    let (kind, label) = kind_for(cmd);
    let shown = compress_and_store(kind, &full, cfg)?;
    stats::record(&label, kind.name(), full.len(), shown.len());
    let mut out = resp.clone();
    out.insert("stdout".into(), Value::String(shown));
    // The full text is now in our store; don't also point the agent at Claude
    // Code's copy, which would invite reading all of it.
    out.remove("persistedOutputPath");
    out.remove("persistedOutputSize");
    Some(Value::Object(out))
}

fn grep(resp: &serde_json::Map<String, Value>, cfg: &Config) -> Option<Value> {
    if resp.get("mode")?.as_str()? != "content" {
        return None;
    }
    let content = resp.get("content")?.as_str()?;
    let shown = compress_and_store(Kind::Search, content, cfg)?;
    stats::record("Grep tool", Kind::Search.name(), content.len(), shown.len());
    let mut out = resp.clone();
    out.insert("content".into(), Value::String(shown));
    Some(Value::Object(out))
}

/// A whole-file Read of a large source file becomes its outline; a large log
/// (`.log`, `.out`) goes through the generic filter. Any Read with `offset` or
/// `limit` is left alone, which is also the agent's way to get exact text.
fn read(input: &Value, resp: &serde_json::Map<String, Value>, cfg: &Config) -> Option<Value> {
    if cfg.read_tool_min_lines == 0
        || input.get("offset").is_some_and(|v| !v.is_null())
        || input.get("limit").is_some_and(|v| !v.is_null())
        || resp.get("type")?.as_str()? != "text"
    {
        return None;
    }
    let file = resp.get("file")?.as_object()?;
    let path = file.get("filePath")?.as_str()?;
    let p = Path::new(path);
    let meta = std::fs::metadata(p).ok()?;
    if !meta.is_file() || meta.len() > 20 * 1024 * 1024 {
        return None;
    }
    let src = String::from_utf8_lossy(&std::fs::read(p).ok()?).into_owned();
    let total = src.lines().count();
    let shown_len = file.get("content")?.as_str()?.len();
    let shown = if let Some(lang) = code::lang_for(p) {
        if total <= cfg.read_tool_min_lines {
            return None;
        }
        let ol = code::outline::outline(&src, lang);
        format!(
            "[ojas: outline of this {total}-line file. Each line starts with its real line number in the file; bodies are elided. For exact text, Read again with offset and limit, e.g. offset=120 limit=60.]\n{ol}"
        )
    } else {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext.to_ascii_lowercase().as_str(), "log" | "out") {
            return None;
        }
        compress_and_store(Kind::Generic, &src, cfg)?
    };
    // An outline only helps if it is clearly smaller than what Read returned.
    if shown.len() * 10 > shown_len * 7 {
        return None;
    }
    stats::record("Read tool", "read", shown_len, shown.len());
    let mut new_file = file.clone();
    new_file.insert("content".into(), Value::String(shown));
    let mut out = resp.clone();
    out.insert("file".into(), Value::Object(new_file));
    Some(Value::Object(out))
}

/// Compress, store the original, and return body + footer. `None` when the
/// output is small, doesn't shrink enough, or can't be stored.
fn compress_and_store(kind: Kind, raw: &str, cfg: &Config) -> Option<String> {
    let body = filters::compress(kind, raw, cfg)?;
    let id = store::put(raw).ok()?;
    let footer = crate::footer(raw, &body, &id);
    store::prune_if_due(cfg.store_days);
    Some(format!("{body}\n{footer}"))
}

fn read_persisted(path: &str) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_PERSISTED {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// True if the command already went through ojas (the PreToolUse rewrite, or the
/// agent calling `ojas get/read/outline` itself), or opted out with `OJAS_RAW=`.
fn already_handled(cmd: &str) -> bool {
    cmd.contains("OJAS_RAW=")
        || cmd.split_whitespace().any(|w| {
            let p = filters::program_name(w.trim_matches(|c| c == '"' || c == '\''));
            p == "ojas" || p == "tsave"
        })
}

/// Pick the filter for a whole shell command line and a label for stats.
///
/// The output mostly comes from the last segment of a chain (`cd x && cargo
/// test`). In a pipeline, a trailing `head`/`tail` keeps the first stage's
/// format, so that stage decides; any other trailing stage (`grep`, `sort`,
/// `awk`, …) reshapes the lines, so the generic filter is the safe choice.
pub fn kind_for(cmd: &str) -> (Kind, String) {
    let last = cmd
        .split(['\n', ';'])
        .flat_map(|s| s.split("&&"))
        .flat_map(|s| s.split("||"))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or("");
    let stages: Vec<&str> = last.split('|').map(str::trim).collect();
    let first = stages.first().copied().unwrap_or("");
    let words: Vec<String> = first
        .split_whitespace()
        .skip_while(|w| is_assignment(w))
        .map(|w| w.trim_matches(|c| c == '"' || c == '\'').to_string())
        .collect();
    let label = filters::label(&words);
    let tail_only = stages[1..].iter().all(|s| {
        matches!(
            s.split_whitespace()
                .next()
                .map(filters::program_name)
                .as_deref(),
            Some("head" | "tail")
        )
    });
    let kind = if tail_only {
        filters::classify(&words).unwrap_or(Kind::Generic)
    } else {
        Kind::Generic
    };
    let label = if label.is_empty() {
        "shell".into()
    } else {
        label
    };
    (kind, label)
}

fn is_assignment(w: &str) -> bool {
    w.split_once('=').is_some_and(|(k, _)| {
        !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_follows_last_segment_and_pipes() {
        assert_eq!(kind_for("cd web && npm test").0, Kind::Test);
        assert_eq!(kind_for("cargo test 2>&1 | tail -50").0, Kind::Test);
        assert_eq!(kind_for("git log --oneline | head -100").0, Kind::GitLog);
        assert_eq!(kind_for("cargo test | grep FAIL").0, Kind::Generic);
        assert_eq!(kind_for("for f in *; do wc -l $f; done").0, Kind::Generic);
        assert_eq!(kind_for("RUST_LOG=debug cargo test").0, Kind::Test);
        assert_eq!(kind_for("cd x && git diff").1, "git diff");
    }

    #[test]
    fn skips_commands_already_through_ojas() {
        assert!(already_handled("ojas run -- cargo test"));
        assert!(already_handled("\"C:/bin/ojas.exe\" get abc123"));
        assert!(already_handled("OJAS_RAW=1 cargo test"));
        assert!(!already_handled("cargo test | tail"));
    }
}
