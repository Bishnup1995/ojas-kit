//! `tsave init` / `tsave uninit`: add or remove our hooks in a Claude Code
//! settings.json, preserving everything else in the file.

use serde_json::{json, Map, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::paths;

pub fn settings_path(project: bool, explicit: Option<PathBuf>) -> PathBuf {
    if let Some(p) = explicit {
        return p;
    }
    if project {
        PathBuf::from(".claude").join("settings.json")
    } else {
        paths::claude_dir().join("settings.json")
    }
}

fn read_settings(path: &Path) -> io::Result<Value> {
    match fs::read_to_string(path) {
        Ok(s) if s.trim().is_empty() => Ok(json!({})),
        Ok(s) => serde_json::from_str(&s).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{} is not valid JSON ({e}); not touching it",
                    path.display()
                ),
            )
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e),
    }
}

fn is_ours(hook: &Value) -> bool {
    hook.get("command")
        .and_then(Value::as_str)
        .is_some_and(|c| {
            c.contains("tsave")
                && (c.contains(" hook pre-tool-use") || c.contains(" hook session-start"))
        })
}

/// Remove our hook entries, dropping groups and events that end up empty.
fn remove_ours(root: &mut Value) -> usize {
    let mut removed = 0;
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return 0;
    };
    for groups in hooks.values_mut() {
        let Some(groups) = groups.as_array_mut() else {
            continue;
        };
        for group in groups.iter_mut() {
            if let Some(list) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                let before = list.len();
                list.retain(|h| !is_ours(h));
                removed += before - list.len();
            }
        }
        groups.retain(|g| {
            g.get("hooks")
                .and_then(Value::as_array)
                .is_none_or(|l| !l.is_empty())
        });
    }
    hooks.retain(|_, v| v.as_array().is_none_or(|a| !a.is_empty()));
    if hooks.is_empty() {
        if let Some(obj) = root.as_object_mut() {
            obj.remove("hooks");
        }
    }
    removed
}

fn exe_for_hooks() -> io::Result<String> {
    let p = std::env::current_exe()?;
    Ok(format!("\"{}\"", p.to_string_lossy().replace('\\', "/")))
}

fn apply_init(root: &mut Value, exe: &str) -> io::Result<()> {
    remove_ours(root);
    let obj = root.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "settings root is not a JSON object",
        )
    })?;
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "\"hooks\" is not a JSON object")
        })?;
    let mut push = |event: &str, group: Value| -> io::Result<()> {
        let list = hooks
            .entry(event)
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("hooks.{event} is not an array"),
                )
            })?;
        list.push(group);
        Ok(())
    };
    push(
        "PreToolUse",
        json!({"matcher": "Bash", "hooks": [{"type": "command", "command": format!("{exe} hook pre-tool-use")}]}),
    )?;
    push(
        "SessionStart",
        json!({"hooks": [{"type": "command", "command": format!("{exe} hook session-start")}]}),
    )?;
    Ok(())
}

fn write_settings(path: &Path, root: &Value) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        let mut bak = path.as_os_str().to_owned();
        bak.push(".tsave-backup");
        fs::copy(path, PathBuf::from(bak))?;
    }
    let text = serde_json::to_string_pretty(root).map_err(io::Error::other)? + "\n";
    fs::write(path, text)
}

pub fn init(path: &Path, dry_run: bool) -> io::Result<String> {
    let mut root = read_settings(path)?;
    apply_init(&mut root, &exe_for_hooks()?)?;
    if dry_run {
        return serde_json::to_string_pretty(&root).map_err(io::Error::other);
    }
    write_settings(path, &root)?;
    Ok(format!(
        "Installed tsave hooks in {}\n  PreToolUse (Bash): wraps noisy commands in `tsave run --`\n  SessionStart: adds the response-style rules and tells the agent about `tsave get`\nA backup of the previous file is next to it (*.tsave-backup). Restart Claude Code to load the hooks.",
        path.display()
    ))
}

pub fn uninit(path: &Path, dry_run: bool) -> io::Result<String> {
    let mut root = read_settings(path)?;
    let removed = remove_ours(&mut root);
    if removed == 0 {
        return Ok(format!("No tsave hooks found in {}", path.display()));
    }
    if dry_run {
        return serde_json::to_string_pretty(&root).map_err(io::Error::other);
    }
    write_settings(path, &root)?;
    Ok(format!(
        "Removed {removed} tsave hook(s) from {}",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent_and_preserves_other_settings() {
        let mut root = json!({
            "model": "opus",
            "hooks": {"PreToolUse": [{"matcher": "Edit", "hooks": [{"type": "command", "command": "fmt.sh"}]}]}
        });
        apply_init(&mut root, "\"/bin/tsave\"").unwrap();
        apply_init(&mut root, "\"/bin/tsave\"").unwrap();
        assert_eq!(root["model"], "opus");
        let pre = root["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 2);
        assert_eq!(pre[0]["hooks"][0]["command"], "fmt.sh");
        assert_eq!(
            pre[1]["hooks"][0]["command"],
            "\"/bin/tsave\" hook pre-tool-use"
        );
        assert_eq!(root["hooks"]["SessionStart"].as_array().unwrap().len(), 1);

        assert_eq!(remove_ours(&mut root), 2);
        assert_eq!(root["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
        assert!(root["hooks"].get("SessionStart").is_none());
    }

    #[test]
    fn uninit_removes_empty_hooks_object() {
        let mut root = json!({});
        apply_init(&mut root, "tsave").unwrap();
        remove_ours(&mut root);
        assert_eq!(root, json!({}));
    }
}
