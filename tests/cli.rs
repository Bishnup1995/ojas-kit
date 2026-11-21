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

fn post_hook(h: &PathBuf, input: &serde_json::Value) -> Option<serde_json::Value> {
    let o = ojas(h, &["hook", "post-tool-use"], Some(&input.to_string()));
    assert!(o.status.success());
    let s = stdout(&o);
    (!s.trim().is_empty()).then(|| serde_json::from_str(&s).unwrap())
}

fn noisy_log(n: usize) -> String {
    let mut s: String = (0..n)
        .map(|i| format!("2026-11-21T10:00:{:02}Z INFO request id={i} ok\n", i % 60))
        .collect();
    s.push_str("2026-11-21T10:09:59Z ERROR payment failed: card declined\n");
    s
}

#[test]
fn post_tool_use_compresses_piped_bash_output_and_keeps_original() {
    let h = home("post-bash");
    let raw = noisy_log(2000);
    let input = serde_json::json!({
        "hook_event_name": "PostToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "cd svc && cat app.log | tail -2001"},
        "tool_response": {"stdout": raw, "stderr": "", "interrupted": false, "isImage": false, "noOutputExpected": false}
    });
    let v = post_hook(&h, &input).expect("should replace output");
    let out = &v["hookSpecificOutput"];
    assert_eq!(out["hookEventName"], "PostToolUse");
    let shown = out["updatedToolOutput"]["stdout"].as_str().unwrap();
    assert!(shown.contains("ERROR payment failed"), "error line kept");
    assert!(shown.len() * 10 < raw.len(), "should cut >90%");
    assert_eq!(
        out["updatedToolOutput"]["interrupted"], false,
        "other fields kept"
    );
    // The footer's id gives back the exact original.
    let id = shown
        .rsplit("get ")
        .next()
        .unwrap()
        .trim_end_matches(']')
        .trim();
    let got = ojas(&h, &["get", id], None);
    assert_eq!(stdout(&got), raw);
}

#[test]
fn post_tool_use_reads_full_persisted_output() {
    let h = home("post-persist");
    let raw = noisy_log(5000);
    let file = h.join("persisted.txt");
    std::fs::write(&file, &raw).unwrap();
    let input = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {"command": "for f in logs/*; do cat $f; done"},
        "tool_response": {"stdout": &raw[..30000], "stderr": "", "interrupted": false, "isImage": false,
            "noOutputExpected": false, "persistedOutputPath": file.to_string_lossy(), "persistedOutputSize": raw.len()}
    });
    let v = post_hook(&h, &input).expect("should replace output");
    let upd = &v["hookSpecificOutput"]["updatedToolOutput"];
    // The error is on the last line, far past the 30k preview: only the full file has it.
    assert!(upd["stdout"]
        .as_str()
        .unwrap()
        .contains("ERROR payment failed"));
    assert!(upd.get("persistedOutputPath").is_none());
}

#[test]
fn post_tool_use_compresses_grep_tool_content() {
    let h = home("post-grep");
    let content: String = (0..400)
        .map(|i| {
            format!(
                "src/mod{}.rs:{}:    let value_{i} = compute();",
                i % 20,
                i + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let input = serde_json::json!({
        "tool_name": "Grep",
        "tool_input": {"pattern": "let value", "output_mode": "content"},
        "tool_response": {"mode": "content", "numFiles": 0, "filenames": [], "content": content, "numLines": 400}
    });
    let v = post_hook(&h, &input).expect("should replace output");
    let upd = &v["hookSpecificOutput"]["updatedToolOutput"];
    assert_eq!(upd["mode"], "content");
    assert!(upd["content"].as_str().unwrap().len() < content.len() / 2);
}

#[test]
fn post_tool_use_leaves_small_wrapped_and_other_output_alone() {
    let h = home("post-skip");
    let small = serde_json::json!({"tool_name": "Bash", "tool_input": {"command": "ls"},
        "tool_response": {"stdout": "a\nb\n", "stderr": ""}});
    assert!(post_hook(&h, &small).is_none());
    let wrapped = serde_json::json!({"tool_name": "Bash", "tool_input": {"command": "ojas run -- cat x.log"},
        "tool_response": {"stdout": noisy_log(2000), "stderr": ""}});
    assert!(post_hook(&h, &wrapped).is_none());
    let glob = serde_json::json!({"tool_name": "Glob", "tool_input": {"pattern": "**"},
        "tool_response": {"filenames": [], "content": noisy_log(2000)}});
    assert!(post_hook(&h, &glob).is_none());
    let garbage = serde_json::json!({"tool_name": "Bash"});
    assert!(post_hook(&h, &garbage).is_none());
}

#[test]
fn post_tool_use_outlines_large_whole_file_reads_only() {
    let h = home("post-read");
    let src: String = (0..700)
        .map(|i| {
            let body: String = (0..8)
                .map(|k| format!("    let v{k} = x + {i} * {k};\n"))
                .collect();
            format!("pub fn helper_{i}(x: u32) -> u32 {{\n{body}}}\n\n")
        })
        .collect();
    let file = h.join("big.rs");
    std::fs::write(&file, &src).unwrap();
    let read = |input: serde_json::Value| {
        serde_json::json!({
            "tool_name": "Read",
            "tool_input": input,
            "tool_response": {"type": "text", "file": {"filePath": file.to_string_lossy(),
                "content": &src, "numLines": 7700, "startLine": 1, "totalLines": 7700}}
        })
    };
    let v = post_hook(
        &h,
        &read(serde_json::json!({"file_path": file.to_string_lossy()})),
    )
    .expect("whole-file read of a big source file should become an outline");
    let f = &v["hookSpecificOutput"]["updatedToolOutput"]["file"];
    let shown = f["content"].as_str().unwrap();
    assert!(shown.starts_with("[ojas: outline of this 7700-line file."));
    assert!(
        shown.contains("12: pub fn helper_1(x: u32) -> u32 {"),
        "real line numbers"
    );
    assert!(shown.len() < src.len() / 2);
    assert_eq!(f["totalLines"], 7700, "other fields kept");

    // Ranged reads are how the agent gets exact text: never touched.
    let ranged =
        read(serde_json::json!({"file_path": file.to_string_lossy(), "offset": 12, "limit": 10}));
    assert!(post_hook(&h, &ranged).is_none());
    let limited = read(serde_json::json!({"file_path": file.to_string_lossy(), "limit": 100000}));
    assert!(post_hook(&h, &limited).is_none());

    // Small source files and non-log text files are left alone.
    let small = h.join("small.rs");
    std::fs::write(&small, "fn a() {}\n").unwrap();
    let v = serde_json::json!({"tool_name": "Read", "tool_input": {"file_path": small.to_string_lossy()},
        "tool_response": {"type": "text", "file": {"filePath": small.to_string_lossy(), "content": "fn a() {}\n"}}});
    assert!(post_hook(&h, &v).is_none());
    let md = h.join("notes.md");
    std::fs::write(&md, &src).unwrap();
    let v = serde_json::json!({"tool_name": "Read", "tool_input": {"file_path": md.to_string_lossy()},
        "tool_response": {"type": "text", "file": {"filePath": md.to_string_lossy(), "content": &src}}});
    assert!(post_hook(&h, &v).is_none());

    // Turned off with read_tool_min_lines = 0.
    std::fs::write(h.join("config.json"), r#"{"read_tool_min_lines": 0}"#).unwrap();
    assert!(post_hook(
        &h,
        &read(serde_json::json!({"file_path": file.to_string_lossy()}))
    )
    .is_none());
}
