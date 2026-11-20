//! Claude Code hook handlers. Every handler fails open: on any problem it prints
//! nothing and exits 0, so the agent's command runs exactly as it would without us.
//!
//! The PreToolUse handler only rewrites the command. It never sets a permission
//! decision, so Claude Code's normal permission checks still apply.

use serde_json::{json, Value};
use std::io::Read;

use crate::{config, rewrite, rules};

pub fn pre_tool_use() {
    if std::env::var_os("TSAVE_DISABLE").is_some() {
        return;
    }
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(&input) else {
        return;
    };
    if let Some(out) = pre_tool_use_response(&v, &bin_name()) {
        println!("{out}");
    }
}

pub fn pre_tool_use_response(v: &Value, bin: &str) -> Option<Value> {
    if v.get("tool_name")?.as_str()? != "Bash" {
        return None;
    }
    let input = v.get("tool_input")?.as_object()?;
    // Background commands are read incrementally; buffering them would hide progress.
    if input.get("run_in_background").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let cmd = input.get("command")?.as_str()?;
    let new_cmd = rewrite::rewrite(cmd, bin)?;
    let mut updated = input.clone();
    updated.insert("command".into(), Value::String(new_cmd));
    Some(json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "updatedInput": updated
        }
    }))
}

pub fn session_start() {
    if std::env::var_os("TSAVE_DISABLE").is_some() {
        return;
    }
    let cfg = config::load();
    let text = rules::session_context(&cfg.style, &bin_name());
    println!(
        "{}",
        json!({
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": text
            }
        })
    );
}

/// How the agent's shell should invoke tsave: `$TSAVE_BIN`, else `tsave` if it is
/// on PATH, else the absolute path of this executable (forward slashes, quoted).
pub fn bin_name() -> String {
    if let Ok(b) = std::env::var("TSAVE_BIN") {
        if !b.is_empty() {
            return b;
        }
    }
    if on_path("tsave") {
        return "tsave".into();
    }
    std::env::current_exe()
        .map(|p| shell_quote(&p.to_string_lossy().replace('\\', "/")))
        .unwrap_or_else(|_| "tsave".into())
}

fn on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path)
        .any(|d| d.join(name).is_file() || d.join(format!("{name}.exe")).is_file())
}

pub fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._-:".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_bash_and_keeps_other_fields() {
        let v = json!({
            "tool_name": "Bash",
            "tool_input": {"command": "cargo test", "description": "Run tests", "timeout": 60000}
        });
        let out = pre_tool_use_response(&v, "tsave").unwrap();
        let ui = &out["hookSpecificOutput"]["updatedInput"];
        assert_eq!(ui["command"], "tsave run -- cargo test");
        assert_eq!(ui["description"], "Run tests");
        assert_eq!(ui["timeout"], 60000);
        assert_eq!(out["hookSpecificOutput"]["hookEventName"], "PreToolUse");
        assert!(out["hookSpecificOutput"]
            .get("permissionDecision")
            .is_none());
    }

    #[test]
    fn ignores_other_tools_and_background() {
        let read = json!({"tool_name": "Read", "tool_input": {"file_path": "x"}});
        assert!(pre_tool_use_response(&read, "tsave").is_none());
        let bg = json!({"tool_name": "Bash", "tool_input": {"command": "cargo test", "run_in_background": true}});
        assert!(pre_tool_use_response(&bg, "tsave").is_none());
        let plain = json!({"tool_name": "Bash", "tool_input": {"command": "echo hi"}});
        assert!(pre_tool_use_response(&plain, "tsave").is_none());
    }

    #[test]
    fn quoting() {
        assert_eq!(
            shell_quote("C:/Users/me/.cargo/bin/tsave.exe"),
            "C:/Users/me/.cargo/bin/tsave.exe"
        );
        assert_eq!(shell_quote("/opt/my tools/tsave"), "'/opt/my tools/tsave'");
    }
}
