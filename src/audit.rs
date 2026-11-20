//! `tsave audit`: read Claude Code transcripts (`~/.claude/projects/**/*.jsonl`) and
//! report real token usage plus where shell output is still costing tokens.
//!
//! Usage numbers come straight from the API usage blocks recorded in the transcripts.
//! Tool-output sizes are measured in characters and shown as ≈tokens (chars / 4).

use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::filters;
use crate::paths;
use crate::rewrite::words;
use crate::stats::{fmt_num, tok};

#[derive(Default)]
struct Usage {
    input: u64,
    cache_write: u64,
    cache_read: u64,
    output: u64,
}

#[derive(Default)]
struct CmdAgg {
    calls: usize,
    chars: usize,
    supported: bool,
}

fn collect_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth > 0 {
                collect_files(&p, depth - 1, out);
            }
        } else if p.extension().is_some_and(|x| x == "jsonl") {
            out.push(p);
        }
    }
}

fn result_chars(content: &Value) -> usize {
    match content {
        Value::String(s) => s.chars().count(),
        Value::Array(items) => items
            .iter()
            .filter_map(|i| i.get("text").and_then(Value::as_str))
            .map(|s| s.chars().count())
            .sum(),
        _ => 0,
    }
}

/// The most relevant segment of a command line: the last one that is not `cd`.
fn command_head(cmd: &str) -> (String, bool, bool) {
    let wrapped = cmd.contains(" run -- ") && cmd.contains("tsave");
    let segments: Vec<&str> = cmd
        .split(['|', ';', '\n'])
        .flat_map(|s| s.split("&&"))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let mut best: Option<Vec<String>> = None;
    for seg in &segments {
        let w = words(seg);
        if w.first().is_some_and(|f| f != "cd") {
            if filters::classify(&w).is_some() {
                best = Some(w);
                break;
            }
            best.get_or_insert(w);
        }
    }
    let w = best.unwrap_or_default();
    let supported = filters::classify(&w).is_some();
    (filters::label(&w), supported, wrapped)
}

pub fn audit(days: u64) -> String {
    let root = paths::claude_dir().join("projects");
    let mut files = Vec::new();
    collect_files(&root, 3, &mut files);
    if files.is_empty() {
        return format!("No Claude Code transcripts found under {}", root.display());
    }
    let cutoff = paths::iso_utc(paths::now_secs().saturating_sub(days * 86_400));

    let mut usage = Usage::default();
    let mut seen_msgs: HashSet<String> = HashSet::new();
    let mut by_cmd: HashMap<String, CmdAgg> = HashMap::new();
    let (mut bash_calls, mut bash_chars) = (0usize, 0usize);
    let (mut wrapped_calls, mut wrapped_chars) = (0usize, 0usize);
    let mut sessions = 0usize;

    for f in &files {
        let Ok(text) = fs::read_to_string(f) else {
            continue;
        };
        let mut commands: HashMap<String, String> = HashMap::new();
        let mut counted = false;
        for line in text.lines() {
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let ts = v.get("timestamp").and_then(Value::as_str).unwrap_or("");
            if ts < cutoff.as_str() {
                continue;
            }
            let Some(msg) = v.get("message") else {
                continue;
            };
            if !counted {
                sessions += 1;
                counted = true;
            }
            let content = msg.get("content").and_then(Value::as_array);
            match v.get("type").and_then(Value::as_str) {
                Some("assistant") => {
                    let id = msg
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    if let Some(u) = msg.get("usage") {
                        if id.is_empty() || seen_msgs.insert(id) {
                            let g = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
                            usage.input += g("input_tokens");
                            usage.cache_write += g("cache_creation_input_tokens");
                            usage.cache_read += g("cache_read_input_tokens");
                            usage.output += g("output_tokens");
                        }
                    }
                    for block in content.into_iter().flatten() {
                        if block.get("type").and_then(Value::as_str) == Some("tool_use")
                            && block.get("name").and_then(Value::as_str) == Some("Bash")
                        {
                            if let (Some(id), Some(cmd)) = (
                                block.get("id").and_then(Value::as_str),
                                block.pointer("/input/command").and_then(Value::as_str),
                            ) {
                                commands.insert(id.to_string(), cmd.to_string());
                            }
                        }
                    }
                }
                Some("user") => {
                    for block in content.into_iter().flatten() {
                        if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                            continue;
                        }
                        let Some(cmd) = block
                            .get("tool_use_id")
                            .and_then(Value::as_str)
                            .and_then(|id| commands.get(id))
                        else {
                            continue;
                        };
                        let chars = block.get("content").map(result_chars).unwrap_or(0);
                        bash_calls += 1;
                        bash_chars += chars;
                        let (head, supported, wrapped) = command_head(cmd);
                        if wrapped {
                            wrapped_calls += 1;
                            wrapped_chars += chars;
                        } else {
                            let a = by_cmd.entry(head).or_default();
                            a.calls += 1;
                            a.chars += chars;
                            a.supported = supported;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let total_in = usage.input + usage.cache_write + usage.cache_read;
    let pct = |n: u64| {
        if total_in == 0 {
            0.0
        } else {
            100.0 * n as f64 / total_in as f64
        }
    };
    let mut s =
        format!("Claude Code usage, last {days} days ({sessions} transcripts with activity)\n\n");
    s.push_str(&format!(
        "  input, uncached   {:>15}  ({:.1}% of input)\n",
        fmt_num(usage.input as usize),
        pct(usage.input)
    ));
    s.push_str(&format!(
        "  cache writes      {:>15}  ({:.1}%)\n",
        fmt_num(usage.cache_write as usize),
        pct(usage.cache_write)
    ));
    s.push_str(&format!(
        "  cache reads       {:>15}  ({:.1}%)\n",
        fmt_num(usage.cache_read as usize),
        pct(usage.cache_read)
    ));
    s.push_str(&format!(
        "  output            {:>15}\n\n",
        fmt_num(usage.output as usize)
    ));
    s.push_str(&format!(
        "Bash results: {} calls, ≈{} tokens of output entered context\n",
        fmt_num(bash_calls),
        fmt_num(tok(bash_chars))
    ));
    s.push_str(&format!(
        "  through tsave: {} calls, ≈{} tokens\n",
        fmt_num(wrapped_calls),
        fmt_num(tok(wrapped_chars))
    ));
    s.push_str(&format!(
        "  not through tsave: {} calls, ≈{} tokens\n\n",
        fmt_num(bash_calls - wrapped_calls),
        fmt_num(tok(bash_chars - wrapped_chars))
    ));

    let mut cmds: Vec<(String, CmdAgg)> = by_cmd.into_iter().collect();
    cmds.sort_by_key(|(_, a)| std::cmp::Reverse(a.chars));
    if !cmds.is_empty() {
        s.push_str("Largest command output not going through tsave:\n");
        s.push_str(&format!(
            "  {:>10} {:>6}  {:<26} {}\n",
            "≈tokens", "calls", "command", "note"
        ));
        for (c, a) in cmds.iter().take(12) {
            let note = if a.supported {
                "filter exists: install the hook (tsave init)"
            } else {
                "no filter yet"
            };
            let name = if c.is_empty() { "(other)" } else { c.as_str() };
            s.push_str(&format!(
                "  {:>10} {:>6}  {:<26} {}\n",
                fmt_num(tok(a.chars)),
                a.calls,
                name,
                note
            ));
        }
    }
    s
}
