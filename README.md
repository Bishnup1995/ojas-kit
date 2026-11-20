# tsave: token saver for AI coding agents

`tsave` cuts the tokens that coding agents (Claude Code first, any agent second) spend on **command output**. It is a single 2 MB Rust binary with no runtime, no proxy, no daemon and no telemetry.

```
$ tsave run -- grep -rn "def " headroom/proxy
1779 matches in 133 files
headroom/proxy/audit.py (3)
  27: def is_auditable_path(path: str) -> bool:
  32: def _client_ip(request: Any) -> str | None:
  37: def record_admin_action(
…
… +1589 more matches in 110 more files
[tsave: 1779 lines → 154, ≈93.7k → ≈2.8k tokens. Full output: tsave get f1b5d0481dd4]
```

## Why command output

In real agent sessions, most of the bill is **input**: tool output that enters the context and is then re-read on every later turn. On one developer's machine, `tsave audit` over 30 days of Claude Code use showed:

- **97.5% of input tokens were cache reads.** Whatever lands in the context gets paid for again on every turn after it.
- **About 120k tokens of shell output entered the context**, mostly from `grep`, `ls`, `find`, `cat` and `sed`.

Rules about response style help too, but less. caveman's own evals found a long "talk terse" rule set beats a plain "Answer concisely." by about 3%, while costing about 1.2k input tokens on every request. So tsave's style rules are about 150 tokens, and most of the effort goes into output filtering.

## How it works

1. **Hook rewrite.** In Claude Code, a `PreToolUse` hook rewrites noisy commands (`cargo test` becomes `tsave run -- cargo test`).
   - Only plain, simple commands are rewritten, or simple segments of `&&` / `||` / `;` chains.
   - Pipes, redirections, subshells, `$(…)` and env-var prefixes are left alone, so the program always receives exactly the same arguments.
   - The hook never makes a permission decision. Your normal permission rules still apply.
2. **Fixed, rule-based filters.** The same input always gives the same output. There are no LLM calls, and replacing the same output gives identical bytes, which keeps prompt caches valid.

   | Filter | Commands | What it does |
   |---|---|---|
   | test | cargo test, pytest, jest, vitest, go test, mocha, dotnet test… | Hides passing tests and build progress. Keeps failures, panics, tracebacks and summaries. |
   | build | cargo build/clippy, npm/pnpm/yarn install, pip, tsc, eslint, ruff, make, gradle, docker build… | Drops progress and noise lines, and folds `npm WARN deprecated` spam into one line. |
   | git-diff | git diff, git show | Keeps changed lines with 1 line of context, collapses lockfiles to a count, adds a summary of files and lines changed. |
   | git-log | git log | One line per commit: hash, refs, date, author, subject. |
   | git-status | git status | Drops `(use "git …")` hints and caps long file lists. |
   | search | grep, rg, ag, git grep | Groups matches by file, shows at most 8 per file and 40 files, and says how many more there are. |
   | listing | find, fd, ls, tree | One line per directory. Hides `node_modules/`, `target/`, `.git/` and similar folders behind a count. Trims `ls -l` to names and sizes. |
   | json | any command whose output is JSON | Minifies, turns arrays of same-shaped objects into `{"_cols":…,"_rows":…}`, and samples long arrays while keeping error-looking items. The result is still valid JSON. |
   | generic | docker logs, kubectl, gh, curl… | Collapses repeated and near-identical lines, then keeps the head, the tail, and error lines. |

   Rules shared by every filter:
   - **Error lines come first.** Truncation gives error lines their own budget, and lines that differ only in numbers are never merged if they look like errors.
   - **Every gap is labelled**, e.g. `… 340 lines omitted (2 error-like, 5 warning-like) …`.
   - **Small output (40 lines or fewer and 3 KB or less) passes through untouched**, as does any output that would shrink by less than 15%.
   - **Exit codes are always preserved.**
3. **Nothing is lost.** Every original is stored locally (`~/.tsave/store`, kept 7 days). The footer names an id, and the agent can run:
   ```
   tsave get <id>                  # everything
   tsave get <id> --grep 'ERROR'   # matching lines, with line numbers
   tsave get <id> --lines 200:260  # a range
   ```
   If the original can't be stored, tsave prints the uncompressed output instead.
4. **Lean session context.** A `SessionStart` hook adds about 150 tokens of response-style rules, plus a note explaining the `[tsave: …]` footer. The text never changes between sessions, so it doesn't break the prompt cache, and it is re-added after compaction.
5. **Outline-first file reads.** A plain `cat <one source file>` longer than 300 lines is rewritten to `tsave read <file>`, which prints an outline instead of the whole file:
   ```
   $ tsave read src/smart_crusher/crusher.rs
   36: use std::sync::Arc;
   … +9 more imports (lines 47-55)
   77: pub struct CrushArrayResult { … 31 lines }
   145: impl SmartCrusher {
   148:   pub fn crush_with_prose_hook(&self, content: &str, query: &str, bias: f64, hook: &ProseHook<'_>) -> CrushResult { … 27 lines }
   …
   [tsave: outline of a 2930-line file, ≈32k → ≈2.7k tokens. Bodies: tsave read <file> --symbol <name>, or --lines START:END. Whole file: --full]
   ```
   - The agent then reads only what it needs: `tsave read <file> --symbol SmartCrusher::crush`, `--symbol Store.Path` (Go receivers work too), or `--lines 140:200`.
   - Function bodies over 200 lines (app factories, closures) also list their nested definitions, which can be read as `--symbol create_app::handler`.
   - Files of 300 lines or fewer, and files in other languages, print in full, byte for byte, exactly like `cat`.
   - `cat` with flags, globs, variables, several files or a pipe is never rewritten.
   - Languages: Rust, Python, JavaScript/TypeScript, Go, Java, C#.
   - **How it parses:** a pure-Rust scanner, not tree-sitter, because tree-sitter grammars are C code and tsave stays free of C dependencies. A lexer removes comments and string contents, including raw strings, template literals with `${…}` nesting, and JS regex literals. Then a brace-aware (or, for Python, indent-aware) pass finds definitions. It is a heuristic for reading, never for editing.
6. **Repo map (opt-in).** `tsave map` lists the most-referenced definitions in the repo:
   - Ranking is by how many *other* files mention each name. Types weigh more than functions, methods and test files less.
   - The output is cut to a token budget (default 1,500).
   - Files come from `git ls-files`, so `.gitignore` is respected.
   - The output is fully sorted and cached by file size and modification time. It is byte-identical until the code changes, which keeps it prompt-cache friendly.
   - Set `"repo_map": true` to add it to the session context. In testing it took 0.64 s on a cold run and 0.2 s from cache on a 2,900-file repository.
7. **`tsave compress-md <file>`** compresses CLAUDE.md, AGENTS.md and similar files. It is rule-based, with no LLM:
   - It shortens wordy phrases ("in order to" becomes "to"), removes filler words ("basically", "really") and hedges, and trims padding from tables.
   - It never touches code blocks, inline code, URLs, paths, headings, front matter or list structure. A validator checks all of these and aborts without writing if anything changed.
   - The original is backed up to `~/.tsave/backups` first. `--dry-run` shows the token estimate, and `--stdout` prints the result. CRLF line endings are kept.
   - Expect small savings: 0–3% on the real docs tested. Files that are already terse gain nothing. Larger cuts (caveman reports about 33%) need an LLM rewrite, which tsave deliberately doesn't do.

## Install

Build from source with Rust 1.87 or later:

```sh
cargo install --path .        # or: cargo install --git <this repo's URL>
tsave init                    # adds the hooks to ~/.claude/settings.json
```

Prebuilt binaries for Linux, macOS and Windows (x86_64 and arm64) are attached to each GitHub release. They are built by `.github/workflows/release.yml` when a `v*` tag is pushed.

`tsave init` merges into your existing settings, keeps your other hooks, and writes a `settings.json.tsave-backup` first. Options:
- `--project` writes to `./.claude/settings.json`.
- `--settings <file>` writes to any file you name.
- `--dry-run` prints the result without writing.

Restart Claude Code after installing. `tsave uninit` removes only tsave's hooks.

> **Windows with the GNU toolchain:** the build works with the minimal `stable-x86_64-pc-windows-gnu` toolchain because tsave has no C dependencies. The MSVC toolchain works as well.

### Other agents (Cursor, Codex, Gemini CLI, …)

Agents without Claude Code hooks can call tsave directly. Paste the output of `tsave rules` into `AGENTS.md`, `.cursor/rules`, `GEMINI.md` or similar. It tells the agent to run noisy commands as `tsave run -- <cmd>`. You can also pipe into a filter:

```sh
some-command 2>&1 | tsave filter --kind test
```

## Measure it

```sh
tsave gain [--days 7]     # per-filter savings recorded by tsave run (estimate: 1 token ≈ 4 bytes)
tsave audit [--days 30]   # real usage from Claude Code transcripts, plus which commands
                          # still send large output without going through tsave
```

On real output from this project's testing:

| Command | Before | After | Saved |
|---|---:|---:|---:|
| `git show` (large commit, 28k lines) | ≈350k tok | ≈10.9k tok | 97% |
| `grep -rn "def " <python package>` | ≈93.7k tok | ≈2.8k tok | 97% |
| `find . -name "*.py"` (1,873 files) | ≈18.3k tok | ≈3.6k tok | 80% |
| `gh api …/issues?per_page=30` (JSON) | ≈62.6k tok | ≈39.6k tok | 37% |

Outlines (`tsave read` / `tsave outline`) on real files:

| File | Lines | Whole file | Outline | Saved |
|---|---:|---:|---:|---:|
| headroom `content_router.py` | 8,856 | ≈108.8k tok | ≈6.3k tok | 95% |
| headroom `proxy/server.py` | 7,916 | ≈92.3k tok | ≈4.1k tok | 96% |
| headroom `smart_crusher/crusher.rs` | 2,930 | ≈32.2k tok | ≈2.7k tok | 92% |
| headroom `log_compressor.rs` | 2,769 | ≈27.8k tok | ≈3.2k tok | 89% |
| caveman `installer/install.js` | 2,981 | ≈37.0k tok | ≈2.0k tok | 95% |
| caveman `ccr/store_sqlite.go` | 1,343 | ≈12.4k tok | ≈1.3k tok | 90% |
| caveman `cli/src/agent-mcp.ts` | 684 | ≈5.9k tok | ≈1.1k tok | 82% |

These counts are byte-based estimates on single samples, not a benchmark of end-to-end agent cost. An outline only saves tokens if the agent then reads a few symbols rather than the whole file.

## Configuration

Settings live in `~/.tsave/config.json`. Every field is optional:

```json
{
  "enabled": true,
  "style": "full",
  "min_lines": 40,
  "min_bytes": 3000,
  "max_lines": 150,
  "diff_max_lines": 400,
  "store_days": 7,
  "read_min_lines": 300,
  "repo_map": false,
  "repo_map_budget": 1500
}
```

- `style` sets the session rules: `full`, `lite`, or `off` (only the note about the `[tsave: …]` footer).
- `read_min_lines`: `cat` of a source file longer than this becomes an outline.
- `repo_map` / `repo_map_budget`: add a repo map of about this many tokens to the session context.

Environment variables:
- `TSAVE_RAW=1 <cmd>` skips one command. The hook doesn't rewrite commands that start with an env assignment.
- `TSAVE_DISABLE=1` makes both hooks do nothing.
- `TSAVE_HOME` moves the data directory.
- `TSAVE_BIN` sets how the agent's shell should call tsave.

To check how a command would be rewritten: `tsave rewrite "cd web && npm test"`.

## Limits (v0.2)

- **Claude Code can't replace a built-in tool's output after it runs.** tsave therefore rewrites the command before it runs. Commands with pipes or redirections are not filtered.
- **Output is buffered**, so long-running commands show nothing until they finish. Background commands and `--watch` / `--follow` / `-f` are never wrapped.
- **Claude Code's built-in Read tool is not intercepted.** Only `cat` through Bash becomes an outline. `sed -n`, `head` and `tail` are left alone.
- **The structure scanner is heuristic.**
  - Unusual syntax (macros that open blocks, C-preprocessor tricks, deeply nested JS object factories) can produce a coarse outline. `--full` and `--lines` always give the exact file.
  - Top-level JS calls with callbacks (`describe(...)`, `app.get(...)`) are elided rather than outlined.
- **The repo map ranks by name references, not resolved symbols.** Two unrelated things with the same name share credit.
- **compress-md saves little on files that are already well written** (see above).

## Roadmap

- Outline support for the Read tool once Claude Code lets hooks replace built-in tool output, plus more languages (Kotlin, Swift, C/C++, Ruby, PHP).
- Filters learned from `tsave audit` ("missed savings") data, and more test and build formats.
- An end-to-end benchmark on real agent tasks: dollars, cache hit rate and pass rate together, including whether outline-first reads cause extra round trips.

## Credits

tsave reimplements ideas; it does not copy code. The ideas come from these projects:
- [caveman](https://github.com/juliusbrussee/caveman): style rules (answer-first, keep negations, plain prose for warnings), and replacements that are always the same so the prompt cache survives.
- [headroom](https://github.com/headroomlabs-ai/headroom): compressors that always keep errors, recoverable compression (CCR), arrays of objects turned into column/row form, and treating the prompt cache as sacred.
- [rtk](https://github.com/rtk-ai/rtk): rewriting commands in a PreToolUse hook, and filters chosen per command.
- [context-mode](https://github.com/mksglu/context-mode): keep raw output out of the context but retrievable.
- [ccusage](https://github.com/ccusage/ccusage): measuring from real transcripts.
- [aider](https://github.com/Aider-AI/aider) repo map, [repomix](https://github.com/yamadashy/repomix) `--compress`, and [serena](https://github.com/oraios/serena) / [codegraph](https://github.com/colbymchenry/codegraph): definitions ranked by references, signature-only views, and reading one symbol instead of a whole file.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
