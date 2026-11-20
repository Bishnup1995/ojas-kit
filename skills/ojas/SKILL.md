---
name: ojas
description: Cut tokens spent on shell output and file reads. Use when running tests, builds, installs, linters, git diff/log/status, grep/rg or find, when reading large source files, or when you need a quick overview of an unfamiliar repo. Requires the `ojas` binary on PATH.
---

# ojas: read less, keep what matters

`ojas` is a local CLI. Check that it exists with `ojas --version`. If it is missing, tell the user to install it from https://github.com/Bishnup1995/ojas-kit/releases, or run `cargo install --git https://github.com/Bishnup1995/ojas-kit`. Do not download it yourself unless they ask.

## Noisy commands

Run tests, builds, installs, linters, `git diff/log/status/show`, `grep`/`rg` and `find`/`ls -R` as:

```sh
ojas run -- <command>
```

- Errors, failures and summaries are kept first. Passing tests, progress lines and repeats are cut. The exit code is unchanged.
- Small output passes through untouched.
- When lines were cut, the output ends with `[ojas: N lines → M, … Full output: ojas get <id>]`. To see more, **don't re-run the command**. Use:
  - `ojas get <id> --grep '<regex>'` for matching lines, with line numbers
  - `ojas get <id> --lines 200:260` for a range
  - `ojas get <id>` for everything
- Don't wrap commands that stream (`--watch`, `-f`, servers) or interactive commands.
- For commands with pipes, filter the end: `<cmd> 2>&1 | ojas filter --kind test` (kinds: test, build, git-diff, git-log, git-status, search, listing, generic).

## Reading code

Use these before opening a whole large file:

```sh
ojas outline <file>                 # imports and signatures with line numbers, bodies elided
ojas read <file> --symbol <name>    # one function or class in full (also Type::method, Class.method)
ojas read <file> --lines 120:180    # an exact range
ojas read <file> --full             # the whole file, when you really need it
```

Outlines cover Rust, Python, JS/TS, Go, Java and C#. For other file types, `ojas read` prints the file unchanged and `ojas outline` refuses.

## Orienting in a repo

```sh
ojas map [--budget 1500]
```

This lists the most-referenced definitions in the repo within a token budget. Run it once at the start of work on an unfamiliar codebase, instead of listing and opening many files.

## Claude Code users

Running `ojas init` once installs hooks that apply all of the above automatically. `ojas audit` then shows how many tokens were saved.
