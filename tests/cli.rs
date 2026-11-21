//! End-to-end tests that drive the real `ojas` binary.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn home(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ojas-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn ojas(home: &PathBuf, args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ojas"))
        .args(args)
        .env("OJAS_HOME", home)
        .env("OJAS_BIN", "ojas")
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
    let o = ojas(&h, &["run", "--", "git", "--version"], None);
    assert!(o.status.success());
    assert!(stdout(&o).starts_with("git version"));

    // git exits 129 for an unknown option; the code must come back unchanged.
    let o = ojas(
        &h,
        &["run", "--", "git", "--definitely-not-an-option"],
        None,
    );
    assert_eq!(o.status.code(), Some(129));
}

#[test]
fn run_reports_missing_command_as_127() {
    let h = home("missing");
    let o = ojas(&h, &["run", "--", "no-such-program-ojas-test"], None);
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
    let o = ojas(&h, &["filter", "--kind", "generic"], Some(&input));
    let out = stdout(&o);
    assert!(out.len() < input.len() / 10, "expected strong compression");
    assert!(out.contains("ERROR database connection lost"));
    let footer = out.lines().last().unwrap();
    assert!(footer.starts_with("[ojas: 2501 lines → "), "{footer}");
    let id = footer.rsplit(' ').next().unwrap().trim_end_matches(']');

    let full = ojas(&h, &["get", id], None);
    assert_eq!(stdout(&full), input);

    let hits = ojas(&h, &["get", id, "--grep", "^ERROR"], None);
    assert_eq!(stdout(&hits).trim(), "2001: ERROR database connection lost");

    let range = ojas(&h, &["get", &id[..6], "--lines", "2:3"], None);
    assert_eq!(
        stdout(&range).trim(),
        "2: INFO request 1 handled in 1ms\n3: INFO request 2 handled in 2ms"
    );
}

#[test]
fn hook_rewrites_known_commands_only() {
    let h = home("hook");
    let o = ojas(
        &h,
        &["hook", "pre-tool-use"],
        Some(
            r#"{"tool_name":"Bash","tool_input":{"command":"cd app && cargo test","description":"tests"}}"#,
        ),
    );
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        v["hookSpecificOutput"]["updatedInput"]["command"],
        "cd app && ojas run -- cargo test"
    );
    assert_eq!(
        v["hookSpecificOutput"]["updatedInput"]["description"],
        "tests"
    );

    let o = ojas(
        &h,
        &["hook", "pre-tool-use"],
        Some(r#"{"tool_name":"Bash","tool_input":{"command":"cargo test | tail -5"}}"#),
    );
    assert!(o.status.success());
    assert!(o.stdout.is_empty());

    // Garbage input must fail open: no output, exit 0.
    let o = ojas(&h, &["hook", "pre-tool-use"], Some("not json"));
    assert!(o.status.success());
    assert!(o.stdout.is_empty());
}

#[test]
fn session_start_emits_context() {
    let h = home("session");
    let o = ojas(&h, &["hook", "session-start"], None);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(ctx.contains("ojas get"));
    assert!(!ctx.contains("Response style"), "style rules are opt-in");

    std::fs::write(h.join("config.json"), r#"{"style": "full"}"#).unwrap();
    let o = ojas(&h, &["hook", "session-start"], None);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
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
    let o = ojas(
        &h,
        &[
            "run", "--", "git", "-C", &repo_s, "ls-files", "--", "*.toml",
        ],
        None,
    );
    assert!(o.status.success());
    assert_eq!(stdout(&o).trim(), "a.toml");
}

fn big_rust_file(dir: &std::path::Path) -> PathBuf {
    let mut src =
        String::from("use std::fmt;\n\npub struct Engine {\n    level: u8,\n}\n\nimpl Engine {\n");
    for i in 0..60 {
        src.push_str(&format!(
            "    pub fn step_{i}(&self, x: u32) -> u32 {{\n        let y = x + {i};\n        let z = y * 2;\n        let w = z - 1;\n        w\n    }}\n\n"
        ));
    }
    src.push_str("}\n");
    let p = dir.join("engine.rs");
    std::fs::write(&p, src).unwrap();
    p
}

#[test]
fn outline_and_symbol_read() {
    let h = home("outline");
    let file = big_rust_file(&h);
    let f = file.to_string_lossy().into_owned();

    let o = ojas(&h, &["outline", &f], None);
    let text = stdout(&o);
    assert!(text.contains("3: pub struct Engine {"), "{text}");
    assert!(
        text.contains("8:   pub fn step_0(&self, x: u32) -> u32 { … 5 lines }"),
        "{text}"
    );
    assert!(!text.contains("let y"));

    // `read` on a large file gives the outline plus a footer; --symbol gives the body.
    let text = stdout(&ojas(&h, &["read", &f], None));
    assert!(text.contains("bodies elided"));
    assert!(text
        .lines()
        .last()
        .unwrap()
        .starts_with("[ojas: outline of a 428-line file"));
    let text = stdout(&ojas(&h, &["read", &f, "--symbol", "Engine::step_1"], None));
    assert!(text.starts_with("── Engine::step_1 ("), "{text}");
    assert!(text.contains("16:         let y = x + 1;"), "{text}");
    let o = ojas(&h, &["read", &f, "--symbol", "nope"], None);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("Symbols: Engine"));

    // Small files and --full pass through byte-for-byte.
    let small = h.join("small.rs");
    std::fs::write(&small, "fn a() {}\r\n").unwrap();
    let o = ojas(&h, &["read", &small.to_string_lossy()], None);
    assert_eq!(o.stdout, b"fn a() {}\r\n");
    let o = ojas(&h, &["read", &f, "--full"], None);
    assert_eq!(o.stdout, std::fs::read(&file).unwrap());
}

#[test]
fn hook_rewrites_cat_of_large_source_file() {
    let h = home("hookcat");
    big_rust_file(&h);
    let input = serde_json::json!({
        "tool_name": "Bash",
        "cwd": h.to_string_lossy(),
        "tool_input": {"command": "cat engine.rs"}
    })
    .to_string();
    let o = ojas(&h, &["hook", "pre-tool-use"], Some(&input));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        v["hookSpecificOutput"]["updatedInput"]["command"],
        "ojas read engine.rs"
    );
}

#[test]
fn repo_map_command() {
    let h = home("map");
    let dir = h.join("proj");
    std::fs::create_dir_all(&dir).unwrap();
    big_rust_file(&dir);
    std::fs::write(
        dir.join("main.rs"),
        "fn main() { let e = Engine { level: 1 }; }\n",
    )
    .unwrap();
    let text = stdout(&ojas(&h, &["map", "--root", &dir.to_string_lossy()], None));
    assert!(
        text.contains("engine.rs:\n  3: pub struct Engine"),
        "{text}"
    );
    // Second run is served from the cache and is identical.
    let again = stdout(&ojas(&h, &["map", "--root", &dir.to_string_lossy()], None));
    assert_eq!(text, again);

    // Opt-in: the SessionStart context includes the map for the session's cwd.
    let input = serde_json::json!({"cwd": dir.to_string_lossy()}).to_string();
    let ctx = |h: &PathBuf| {
        let o = ojas(h, &["hook", "session-start"], Some(&input));
        let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        v["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert!(!ctx(&h).contains("Repo map"), "off by default");
    std::fs::write(h.join("config.json"), r#"{"repo_map": true}"#).unwrap();
    assert!(ctx(&h).contains("Repo map (ojas)"));
}

#[test]
fn compress_md_backs_up_and_validates() {
    let h = home("md");
    let file = h.join("CLAUDE.md");
    let src = "# Project\n\nIn order to run the tests, you should basically use `cargo test`.\n\n```sh\njust keep this\n```\n";
    std::fs::write(&file, src).unwrap();
    let f = file.to_string_lossy().into_owned();

    let o = ojas(&h, &["compress-md", &f, "--dry-run"], None);
    assert!(stdout(&o).contains("dry run, nothing written"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), src);

    let o = ojas(&h, &["compress-md", &f], None);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "# Project\n\nTo run the tests, you should use `cargo test`.\n\n```sh\njust keep this\n```\n"
    );
    let backups: Vec<_> = std::fs::read_dir(h.join("backups")).unwrap().collect();
    assert_eq!(backups.len(), 1);
    let saved = std::fs::read_to_string(backups[0].as_ref().unwrap().path()).unwrap();
    assert_eq!(saved, src);

    let o = ojas(
        &h,
        &["compress-md", &h.join("x.rs").to_string_lossy()],
        None,
    );
    assert_eq!(o.status.code(), Some(2));
}
