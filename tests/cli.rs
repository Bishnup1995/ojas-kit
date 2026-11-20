//! End-to-end tests that drive the real `tsave` binary.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn home(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("tsave-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn tsave(home: &PathBuf, args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tsave"))
        .args(args)
        .env("TSAVE_HOME", home)
        .env("TSAVE_BIN", "tsave")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut si = child.stdin.take().unwrap();
        if let Some(s) = stdin {
            si.write_all(s.as_bytes()).unwrap();
        }
    }
    child.wait_with_output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn run_passes_small_output_and_exit_code_through() {
    let h = home("small");
    let o = tsave(&h, &["run", "--", "git", "--version"], None);
    assert!(o.status.success());
    assert!(stdout(&o).starts_with("git version"));

    // git exits 129 for an unknown option; the code must come back unchanged.
    let o = tsave(
        &h,
        &["run", "--", "git", "--definitely-not-an-option"],
        None,
    );
    assert_eq!(o.status.code(), Some(129));
}

#[test]
fn run_reports_missing_command_as_127() {
    let h = home("missing");
    let o = tsave(&h, &["run", "--", "no-such-program-tsave-test"], None);
    assert_eq!(o.status.code(), Some(127));
}

#[test]
fn filter_then_get_round_trips_the_original() {
    let h = home("roundtrip");
    let mut input = String::new();
    for i in 0..2000 {
        input.push_str(&format!("INFO request {i} handled in {}ms\n", i % 50));
    }
    input.push_str("ERROR database connection lost\n");
    for i in 0..500 {
        input.push_str(&format!("INFO request {i} retried\n"));
    }
    let o = tsave(&h, &["filter", "--kind", "generic"], Some(&input));
    let out = stdout(&o);
    assert!(out.len() < input.len() / 10, "expected strong compression");
    assert!(out.contains("ERROR database connection lost"));
    let footer = out.lines().last().unwrap();
    assert!(footer.starts_with("[tsave: 2501 lines → "), "{footer}");
    let id = footer.rsplit(' ').next().unwrap().trim_end_matches(']');

    let full = tsave(&h, &["get", id], None);
    assert_eq!(stdout(&full), input);

    let hits = tsave(&h, &["get", id, "--grep", "^ERROR"], None);
    assert_eq!(stdout(&hits).trim(), "2001: ERROR database connection lost");

    let range = tsave(&h, &["get", &id[..6], "--lines", "2:3"], None);
    assert_eq!(
        stdout(&range).trim(),
        "2: INFO request 1 handled in 1ms\n3: INFO request 2 handled in 2ms"
    );
}

#[test]
fn hook_rewrites_known_commands_only() {
    let h = home("hook");
    let o = tsave(
        &h,
        &["hook", "pre-tool-use"],
        Some(
            r#"{"tool_name":"Bash","tool_input":{"command":"cd app && cargo test","description":"tests"}}"#,
        ),
    );
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        v["hookSpecificOutput"]["updatedInput"]["command"],
        "cd app && tsave run -- cargo test"
    );
    assert_eq!(
        v["hookSpecificOutput"]["updatedInput"]["description"],
        "tests"
    );

    let o = tsave(
        &h,
        &["hook", "pre-tool-use"],
        Some(r#"{"tool_name":"Bash","tool_input":{"command":"cargo test | tail -5"}}"#),
    );
    assert!(o.status.success());
    assert!(o.stdout.is_empty());

    // Garbage input must fail open: no output, exit 0.
    let o = tsave(&h, &["hook", "pre-tool-use"], Some("not json"));
    assert!(o.status.success());
    assert!(o.stdout.is_empty());
}

#[test]
fn session_start_emits_context() {
    let h = home("session");
    let o = tsave(&h, &["hook", "session-start"], None);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(ctx.contains("tsave get"));
    assert!(ctx.contains("Lead with the answer"));
}

#[test]
fn glob_arguments_reach_the_program_unexpanded() {
    // Regression test: on Windows, MSYS/MinGW programs expand unquoted `*` themselves,
    // relative to their working directory. The test's working directory holds
    // Cargo.toml, so an expanded glob would make git look for the wrong file.
    let h = home("glob");
    let repo = h.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("a.toml"), "x").unwrap();
    std::fs::write(repo.join("b.txt"), "x").unwrap();
    let git = |args: &[&str]| {
        let s = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(args)
            .output()
            .unwrap();
        assert!(s.status.success(), "{:?}", s);
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    let repo_s = repo.to_string_lossy().into_owned();
    let o = tsave(
        &h,
        &[
            "run", "--", "git", "-C", &repo_s, "ls-files", "--", "*.toml",
        ],
        None,
    );
    assert!(o.status.success());
    assert_eq!(stdout(&o).trim(), "a.toml");
}
