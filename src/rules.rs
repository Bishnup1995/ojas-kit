//! The text injected into the agent's context.
//!
//! Kept short on purpose: caveman's evals found a long rule set beats a plain
//! "be concise" by only a few percent while costing ~1.2k input tokens on every
//! request. These rules are about 150 tokens, and opt-in: in our own benchmark
//! (Claude Code, 8 tasks x 3 runs) they added turns and cost without shortening
//! answers, so the default (`off`) injects only the note about the output footer.
//! The text is fixed (no dates, no
//! counters) so it never breaks the prompt-cache prefix.

const STYLE_FULL: &str = "\
Response style (token-saver):
- Lead with the answer or result. No preamble, no recap, no \"Sure\", \"Great question\", \"Let me\", \"I'll now\".
- Short, plain sentences. Cut filler (just, really, basically, actually, simply). Never cut negations or qualifiers (not, never, only, except, unless).
- Keep code, commands, paths, error text and quotes exact. Never abbreviate them.
- To show a code change in chat, show only the changed lines plus 1-2 lines of context. Don't echo code already written to a file.
- No narration between routine tool calls.
- Use full, clear prose for security warnings, irreversible actions, ordered steps, questions to the user, and anything written to files, commits or PRs.";

const STYLE_LITE: &str = "\
Response style (token-saver): be concise. Lead with the answer, skip preamble and recap, and keep code, commands and error text exact.";

fn tool_note(bin: &str) -> String {
    format!(
        "\
Command output (token-saver): some shell commands are run through `ojas run --`, which shortens long output. Error and failure lines are kept first. A line like `[ojas: 2140 lines → 96, ≈21k → ≈1.1k tokens. Full output: {bin} get 3fa9c1d2e4b5]` means lines were omitted. To see more, run `{bin} get <id> --grep <regex>` or `{bin} get <id> --lines 200:260` instead of re-running the command. To skip compression for one command, prefix it with `OJAS_RAW=1 `. `cat` of a large source file shows an outline; get bodies with `{bin} read <file> --symbol <name>` or `--lines A:B`."
    )
}

/// Context for the Claude Code SessionStart hook.
pub fn session_context(style: &str, bin: &str) -> String {
    match style {
        "full" => format!("{STYLE_FULL}\n\n{}", tool_note(bin)),
        "lite" => format!("{STYLE_LITE}\n\n{}", tool_note(bin)),
        _ => tool_note(bin),
    }
}

/// Rules for agents without hooks (Cursor, Codex, Gemini CLI, ...): paste into
/// AGENTS.md, .cursor/rules or similar.
pub fn agent_rules(style: &str) -> String {
    let style_text = match style {
        "full" => format!("{STYLE_FULL}\n\n"),
        "lite" => format!("{STYLE_LITE}\n\n"),
        _ => String::new(),
    };
    format!(
        "{style_text}\
Command output (token-saver): run noisy commands (tests, builds, installs, linters, git diff/log/status, grep/rg, find) as `ojas run -- <command>`. It keeps errors and failures and shortens the rest. When output ends with `[ojas: … Full output: ojas get <id>]`, use `ojas get <id> --grep <regex>` or `ojas get <id> --lines 200:260` to see more instead of re-running the command."
    )
}
