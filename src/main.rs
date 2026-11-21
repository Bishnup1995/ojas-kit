//! ojas: cut the tokens AI coding agents spend on command output.

mod audit;
mod code;
mod codecmd;
mod config;
mod filters;
mod hook;
mod install;
mod mdcompress;
mod paths;
mod posthook;
mod repomap;
mod rewrite;
mod rules;
mod runner;
mod stats;
mod store;

use clap::{Parser, Subcommand};
use std::io::{self, Read, Write};
use std::path::PathBuf;

use filters::Kind;

#[derive(Parser)]
#[command(
    name = "ojas",
    version,
    about = "Cut the tokens AI coding agents spend on command output"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a command and print its output, compressed when that clearly helps
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
    },
    /// Print a stored original output by id
    Get {
        id: String,
        /// Only print lines matching this regex (with line numbers)
        #[arg(long)]
        grep: Option<String>,
        /// Only print a line range, e.g. 200:260, 200: or :50 (1-based, inclusive)
        #[arg(long)]
        lines: Option<String>,
    },
    /// Compress text from stdin (for pipes and agents without hooks)
    Filter {
        /// git-status, git-diff, git-log, test, search, listing, build or generic
        #[arg(long, default_value = "generic")]
        kind: String,
    },
    /// Claude Code hook entry points (called by Claude Code, not by people)
    Hook {
        #[command(subcommand)]
        event: HookEvent,
    },
    /// Install the hooks into Claude Code settings (global by default)
    Init(InstallArgs),
    /// Remove the hooks from Claude Code settings
    Uninit(InstallArgs),
    /// Show estimated savings from recorded runs
    Gain {
        /// Only count the last N days
        #[arg(long)]
        days: Option<u64>,
    },
    /// Read Claude Code transcripts: real token usage and output still not compressed
    Audit {
        #[arg(long, default_value_t = 30)]
        days: u64,
    },
    /// Print rules to paste into AGENTS.md, Cursor rules, etc. for agents without hooks
    Rules {
        /// off (default: only the ojas usage note), lite or full
        #[arg(long, default_value = "off")]
        style: String,
    },
    /// Show how the hook would rewrite a shell command (for debugging)
    Rewrite { command: String },
    /// Print a source file's outline: imports, declarations, signatures; bodies elided
    Outline { file: PathBuf },
    /// Read a source file: outline for large files, full text for small ones
    Read {
        file: PathBuf,
        /// Print only this symbol's full source (`name`, `Type::method` or `Class.method`)
        #[arg(long)]
        symbol: Option<String>,
        /// Print a line range, e.g. 120:180 (1-based, inclusive)
        #[arg(long)]
        lines: Option<String>,
        /// Print the whole file regardless of size
        #[arg(long)]
        full: bool,
    },
    /// Print a repo map: the most-referenced definitions, cut to a token budget
    Map {
        /// Approximate token budget
        #[arg(long, default_value_t = 1500)]
        budget: usize,
        /// Repository root (default: current directory)
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Compress a CLAUDE.md / AGENTS.md style file in place (rule-based, validated, backed up)
    CompressMd {
        file: PathBuf,
        /// Only report the token estimate; write nothing
        #[arg(long)]
        dry_run: bool,
        /// Print the compressed text instead of writing the file
        #[arg(long)]
        stdout: bool,
    },
}

#[derive(Subcommand)]
enum HookEvent {
    PreToolUse,
    PostToolUse,
    SessionStart,
}

#[derive(clap::Args)]
struct InstallArgs {
    /// Use ./.claude/settings.json instead of the global settings
    #[arg(long)]
    project: bool,
    /// Use this settings file
    #[arg(long, conflicts_with = "project")]
    settings: Option<PathBuf>,
    /// Print the resulting settings without writing
    #[arg(long)]
    dry_run: bool,
}

/// Write to stdout, ignoring broken pipes (`ojas get x | head`).
fn out(s: &str) {
    let mut o = io::stdout().lock();
    let _ = o.write_all(s.as_bytes());
    if !s.ends_with('\n') && !s.is_empty() {
        let _ = o.write_all(b"\n");
    }
    let _ = o.flush();
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.cmd {
        Cmd::Run { command } => cmd_run(command),
        Cmd::Get { id, grep, lines } => cmd_get(&id, grep, lines),
        Cmd::Filter { kind } => cmd_filter(&kind),
        Cmd::Hook { event } => {
            match event {
                HookEvent::PreToolUse => hook::pre_tool_use(),
                HookEvent::PostToolUse => posthook::post_tool_use(),
                HookEvent::SessionStart => hook::session_start(),
            }
            0
        }
        Cmd::Init(a) => report(install::init(
            &install::settings_path(a.project, a.settings),
            a.dry_run,
        )),
        Cmd::Uninit(a) => report(install::uninit(
            &install::settings_path(a.project, a.settings),
            a.dry_run,
        )),
        Cmd::Gain { days } => {
            out(&stats::gain(days));
            0
        }
        Cmd::Audit { days } => {
            out(&audit::audit(days));
            0
        }
        Cmd::Rules { style } => {
            out(&rules::agent_rules(&style));
            0
        }
        Cmd::Rewrite { command } => {
            let cwd = std::env::current_dir().ok();
            let min = config::load().read_min_lines;
            match rewrite::rewrite_ctx(&command, &hook::bin_name(), cwd.as_deref(), min) {
                Some(r) => out(&r),
                None => out("(unchanged)"),
            }
            0
        }
        Cmd::Outline { file } => codecmd::outline(&file),
        Cmd::Read {
            file,
            symbol,
            lines,
            full,
        } => codecmd::read(&file, symbol.as_deref(), lines.as_deref(), full),
        Cmd::Map { budget, root } => codecmd::map(&root, budget),
        Cmd::CompressMd {
            file,
            dry_run,
            stdout,
        } => codecmd::compress_md(&file, dry_run, stdout),
    };
    std::process::exit(code);
}

fn report(r: io::Result<String>) -> i32 {
    match r {
        Ok(msg) => {
            out(&msg);
            0
        }
        Err(e) => {
            eprintln!("ojas: {e}");
            1
        }
    }
}

fn cmd_run(command: Vec<String>) -> i32 {
    let cfg = config::load();
    let captured = match runner::run(&command) {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            eprintln!("ojas: {}: command not found", command[0]);
            return 127;
        }
        Err(e) => {
            eprintln!("ojas: could not run {}: {e}", command[0]);
            return 126;
        }
    };
    let raw = &captured.text;
    let kind = filters::classify(&command).unwrap_or(Kind::Generic);
    let passthrough = !cfg.enabled || std::env::var_os("OJAS_RAW").is_some();
    let compressed = if passthrough {
        None
    } else {
        filters::compress(kind, raw, &cfg)
    };
    // Only show compressed output if the original is safely stored.
    let shown = compressed.and_then(|body| {
        let id = store::put(raw).ok()?;
        Some(format!("{body}\n{}\n", footer(raw, &body, &id)))
    });
    match &shown {
        Some(s) => out(s),
        None => {
            let mut o = io::stdout().lock();
            let _ = o.write_all(raw.as_bytes());
            let _ = o.flush();
        }
    }
    if !raw.is_empty() {
        let sent = shown.as_ref().map_or(raw.len(), String::len);
        stats::record(&filters::label(&command), kind.name(), raw.len(), sent);
    }
    store::prune_if_due(cfg.store_days);
    captured.code
}

/// `1.2k`-style compact numbers for the footer.
fn short_num(n: usize) -> String {
    match n {
        0..=999 => n.to_string(),
        1_000..=999_999 => format!("{:.1}k", n as f64 / 1e3),
        _ => format!("{:.1}M", n as f64 / 1e6),
    }
}

/// The last line of compressed output: what was cut and how to get it back.
pub(crate) fn footer(raw: &str, body: &str, id: &str) -> String {
    format!(
        "[ojas: {} lines → {}, ≈{} → ≈{} tokens. Full output: {} get {id}]",
        raw.lines().count(),
        body.lines().count(),
        short_num(stats::tok(raw.len())),
        short_num(stats::tok(body.len())),
        hook::bin_name()
    )
}

pub(crate) fn parse_range(spec: &str, len: usize) -> Option<(usize, usize)> {
    let (a, b) = spec.split_once(':')?;
    let start = if a.trim().is_empty() {
        1
    } else {
        a.trim().parse().ok()?
    };
    let end = if b.trim().is_empty() {
        len
    } else {
        b.trim().parse().ok()?
    };
    let end = end.min(len);
    (start >= 1 && start <= end).then_some((start, end))
}

fn cmd_get(id: &str, grep: Option<String>, lines: Option<String>) -> i32 {
    let text = match store::get(id) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("ojas: {e}");
            return 1;
        }
    };
    let all: Vec<&str> = text.lines().collect();
    if let Some(pat) = grep {
        let re = match regex::Regex::new(&pat) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("ojas: invalid regex: {e}");
                return 2;
            }
        };
        const MAX_HITS: usize = 300;
        let hits: Vec<String> = all
            .iter()
            .enumerate()
            .filter(|(_, l)| re.is_match(l))
            .map(|(i, l)| format!("{}: {l}", i + 1))
            .collect();
        if hits.is_empty() {
            eprintln!("ojas: no lines match");
            return 1;
        }
        let mut s = hits
            .iter()
            .take(MAX_HITS)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if hits.len() > MAX_HITS {
            s.push_str(&format!(
                "\n… {} more matching lines (narrow the regex)",
                hits.len() - MAX_HITS
            ));
        }
        out(&s);
        return 0;
    }
    if let Some(spec) = lines {
        let Some((start, end)) = parse_range(&spec, all.len()) else {
            eprintln!(
                "ojas: invalid range '{spec}' (expected START:END, the output has {} lines)",
                all.len()
            );
            return 2;
        };
        let s: Vec<String> = (start..=end)
            .map(|i| format!("{i}: {}", all[i - 1]))
            .collect();
        out(&s.join("\n"));
        return 0;
    }
    out(&text);
    0
}

fn cmd_filter(kind: &str) -> i32 {
    let Some(kind) = Kind::parse(kind) else {
        eprintln!("ojas: unknown kind '{kind}'");
        return 2;
    };
    let mut raw = String::new();
    if io::stdin().read_to_string(&mut raw).is_err() {
        eprintln!("ojas: could not read stdin as UTF-8");
        return 1;
    }
    let cfg = config::load();
    match filters::compress(kind, &raw, &cfg) {
        Some(body) => match store::put(&raw) {
            Ok(id) => out(&format!("{body}\n{}", footer(&raw, &body, &id))),
            Err(_) => out(&raw),
        },
        None => out(&raw),
    }
    0
}

#[cfg(test)]
mod tests {
    use super::parse_range;

    #[test]
    fn ranges() {
        assert_eq!(parse_range("2:5", 10), Some((2, 5)));
        assert_eq!(parse_range(":3", 10), Some((1, 3)));
        assert_eq!(parse_range("8:", 10), Some((8, 10)));
        assert_eq!(parse_range("8:99", 10), Some((8, 10)));
        assert_eq!(parse_range("5:2", 10), None);
        assert_eq!(parse_range("0:2", 10), None);
    }
}
