---
name: ojas
description: Cut tokens spent on shell output and file reads without losing errors. Use when running test suites, builds, installs, linters, git diff/log/status, grep/rg or find, when reading large log files or large source files, or when you need a quick overview of an unfamiliar repo. Requires the `ojas` binary on PATH.
---

# ojas: read less, keep what matters

`ojas` is a local CLI. Check that it exists with `ojas --version`. If it is missing, tell the user to install it from https://github.com/Bishnup1995/ojas-kit/releases, or run `cargo install --git https://github.com/Bishnup1995/ojas-kit`. Do not download it yourself unless they ask.

## Noisy commands

Run tests, builds, installs, linters, `git diff/log/status/show`, `grep`/`rg`, `find`/`ls -R`, and `cat` of large log or text files as:

```sh
ojas run -- <command>
```

- Errors, failures and summaries are kept first. Passing tests, progress lines and repeated log lines are cut. The exit code is unchanged.
- Small output passes through untouched.
- When lines were cut, the output ends with `[ojas: N lines → M, … Full output: ojas get <id>]`. To see more, **don't re-run the command**. Use:
  - `ojas get <id> --grep '<regex>'` for matching lines, with line numbers
  - `ojas get <id> --lines 200:260` for a range
  - `ojas get <id>` for everything
- Don't wrap commands that stream (`--watch`, `-f`, servers) or interactive commands.

## Don't cut test, build or log output yourself

Don't pipe a test suite, build or log through `| head` or `| tail` to keep it short. The failure is often in the part you cut, and agent harnesses drop the middle of very long output too. Use `ojas run --` instead, which keeps failures wherever they are. In benchmark runs on a full express test suite, agents without ojas missed the failing test in every run, and agents with it found it in every run.

If you already narrowed the output to what you need (`grep` for one exact name, `sed -n 40,80p`), ojas adds nothing, so run it plainly. For other pipelines, filter the end: `<cmd> 2>&1 | ojas filter --kind test` (kinds: test, build, git-diff, git-log, git-status, search, listing, generic).

## Reading code

Pick the approach by task.

**Finding and fixing something specific** (one bug, one function, one call site): start with the outline, then read only what you need.

```sh
ojas outline <file>                 # imports and signatures with real line numbers, bodies elided
ojas read <file> --symbol <name>    # one function or class in full (also Type::method, Class.method)
ojas read <file> --lines 120:180    # an exact range
```

**Needing every body** (trace every caller or every error path, review or rewrite a whole module): read the whole file once with `ojas read <file> --full`, or your normal file reader. Opening an outline first and then most of the symbols one by one costs more than reading the file once.

Outlines cover Rust, Python, JS/TS, Go, Java and C#. For other file types, `ojas read` prints the file unchanged and `ojas outline` refuses.

## Orienting in a repo

```sh
ojas map [--budget 1500]
```

This lists the most-referenced definitions in the repo within a token budget. Run it once at the start of work on an unfamiliar codebase, instead of listing and opening many files.

## Claude Code users

Running `ojas init` once installs hooks that apply all of the above automatically, including for pipes, the Grep tool and whole-file Reads. `ojas audit` then shows how many tokens were saved.
