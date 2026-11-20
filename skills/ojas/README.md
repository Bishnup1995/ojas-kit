# ojas skill

This agent skill teaches coding agents (Claude Code, Cursor, Codex, Copilot, Gemini CLI and others) to use **ojas**. ojas is a free, open-source CLI that cuts the tokens spent on command output and file reads.

## Requirement: the `ojas` binary

The skill only holds instructions. The work is done by a local binary that must be on your PATH.

- **Prebuilt** (Windows, macOS and Linux, x86_64 and arm64): download from https://github.com/Bishnup1995/ojas-kit/releases and put `ojas` on your PATH.
- **From source** (Rust 1.87 or later): `cargo install --git https://github.com/Bishnup1995/ojas-kit`

Check that it works with `ojas --version`.

## What the agent gets

- `ojas run -- <cmd>`: test, build, git and search output with failures kept and noise cut. In testing it saved 80–97% on large outputs. Nothing is lost: the full output stays retrievable with `ojas get <id>`.
- `ojas outline` / `ojas read --symbol`: signatures first, function bodies only on demand. One 435 KB file became 25 KB.
- `ojas map`: an overview of a repo's key definitions, limited to a token budget.

## Claude Code: full automation

Run `ojas init` once. Hooks then rewrite noisy commands automatically, with no instructions needed. `ojas uninit` removes them.

License: MIT OR Apache-2.0. Source: https://github.com/Bishnup1995/ojas-kit
