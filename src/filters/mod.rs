//! Command classification and the compression pipeline.
//!
//! `classify` decides whether a command is worth wrapping (the hook uses it) and which
//! filter to use (`tsave run` uses it). `compress` returns `None` whenever compressing
//! would not clearly help, so callers can pass the original through untouched.

pub mod build;
pub mod common;
pub mod git;
pub mod json;
pub mod listing;
pub mod search;
pub mod test;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    GitStatus,
    GitDiff,
    GitLog,
    Test,
    Search,
    Listing,
    Build,
    Generic,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::GitStatus => "git-status",
            Kind::GitDiff => "git-diff",
            Kind::GitLog => "git-log",
            Kind::Test => "test",
            Kind::Search => "search",
            Kind::Listing => "listing",
            Kind::Build => "build",
            Kind::Generic => "generic",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        [
            Kind::GitStatus,
            Kind::GitDiff,
            Kind::GitLog,
            Kind::Test,
            Kind::Search,
            Kind::Listing,
            Kind::Build,
            Kind::Generic,
        ]
        .into_iter()
        .find(|k| k.name() == s)
    }
}

/// Compress `raw` output of a command of the given kind. Returns `None` when the
/// output is small or compression saves less than 15%.
pub fn compress(kind: Kind, raw: &str, cfg: &Config) -> Option<String> {
    if raw.lines().count() <= cfg.min_lines && raw.len() <= cfg.min_bytes {
        return None;
    }
    let out = if let Some(v) = json::parse(raw) {
        json::compress(v)
    } else {
        let text = common::clean(raw);
        match kind {
            Kind::GitStatus => common::finalize(&git::status(&text), cfg.max_lines, false),
            Kind::GitDiff => common::finalize(&git::diff(&text), cfg.diff_max_lines, false),
            Kind::GitLog => common::finalize(&git::log(&text), cfg.max_lines, false),
            Kind::Test => common::finalize(&test::compress(&text), cfg.max_lines, true),
            Kind::Search => search::compress(&text, cfg.max_lines),
            Kind::Listing => common::finalize(&listing::compress(&text), cfg.max_lines, false),
            Kind::Build => common::finalize(&build::compress(&text), cfg.max_lines, true),
            Kind::Generic => common::finalize(&text, cfg.max_lines, true),
        }
    };
    if out.len() * 100 >= raw.len() * 85 {
        None
    } else {
        Some(out)
    }
}

/// Basename of a program without directory or Windows extension, lowercased.
pub fn program_name(word: &str) -> String {
    let base = word.rsplit(['/', '\\']).next().unwrap_or(word);
    let lower = base.to_ascii_lowercase();
    for ext in [".exe", ".cmd", ".bat", ".ps1"] {
        if let Some(stem) = lower.strip_suffix(ext) {
            return stem.to_string();
        }
    }
    lower
}

/// Skip launchers such as `npx`, `uv run`, `python -m` to reach the real tool.
fn strip_runners<'a, 'b>(mut w: &'a [&'b str]) -> &'a [&'b str] {
    loop {
        let Some(first) = w.first() else { return w };
        let first = program_name(first);
        let second = w.get(1).copied().unwrap_or("");
        let skip = match first.as_str() {
            "npx" | "bunx" | "pnpx" => {
                let mut i = 1;
                while w.get(i).is_some_and(|a| a.starts_with('-')) {
                    i += 1;
                }
                i
            }
            "uv" | "poetry" | "pipenv" | "pdm" | "hatch" | "rye" if second == "run" => 2,
            "pnpm" | "yarn" if second == "exec" || second == "dlx" => 2,
            "bundle" if second == "exec" => 2,
            "python" | "python3" | "py" if second == "-m" => 2,
            _ => 0,
        };
        if skip == 0 || skip >= w.len() {
            return w;
        }
        w = &w[skip..];
    }
}

fn first_positional<'a>(args: &[&'a str]) -> Option<&'a str> {
    args.iter()
        .copied()
        .find(|a| !a.starts_with('-') && !a.starts_with('+'))
}

/// `git -C dir --no-pager log` -> `log`.
fn git_subcommand<'a>(args: &[&'a str]) -> &'a str {
    let mut i = 0;
    while i < args.len() {
        let a = args[i];
        match a {
            "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace" => i += 2,
            _ if a.starts_with('-') => i += 1,
            _ => return a,
        }
    }
    ""
}

fn script_kind(name: &str) -> Option<Kind> {
    let n = name.to_ascii_lowercase();
    if ["test", "spec", "e2e"].iter().any(|k| n.contains(k)) {
        Some(Kind::Test)
    } else if ["build", "lint", "check", "typecheck", "compile", "tsc"]
        .iter()
        .any(|k| n.contains(k))
    {
        Some(Kind::Build)
    } else {
        None
    }
}

/// Short label for stats, e.g. `cargo test` or `git diff`.
pub fn label<S: AsRef<str>>(words: &[S]) -> String {
    let all: Vec<&str> = words.iter().map(|w| w.as_ref()).collect();
    let w = strip_runners(&all);
    let Some(first) = w.first() else {
        return String::new();
    };
    let prog = program_name(first);
    // Only tools with real subcommands get a second word (`ls src` is not a subcommand).
    const HAS_SUBCOMMANDS: &[&str] = &[
        "git",
        "cargo",
        "go",
        "npm",
        "pnpm",
        "yarn",
        "bun",
        "pip",
        "pip3",
        "uv",
        "poetry",
        "docker",
        "podman",
        "kubectl",
        "oc",
        "dotnet",
        "gh",
        "terraform",
        "tofu",
        "gradle",
        "gradlew",
        "mvn",
        "mvnw",
        "bazel",
        "swift",
        "playwright",
    ];
    if !HAS_SUBCOMMANDS.contains(&prog.as_str()) {
        return prog;
    }
    let rest = &w[1..];
    let sub = if prog == "git" {
        git_subcommand(rest)
    } else {
        first_positional(rest).unwrap_or("")
    };
    let sub_is_word = !sub.is_empty()
        && sub
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == ':');
    if sub_is_word && sub.len() <= 20 {
        format!("{prog} {sub}")
    } else {
        prog
    }
}

/// Decide whether a command has a filter, and which one.
pub fn classify<S: AsRef<str>>(words: &[S]) -> Option<Kind> {
    use Kind::*;
    let all: Vec<&str> = words.iter().map(|w| w.as_ref()).collect();
    // Never buffer commands that are meant to stream forever.
    if all.iter().any(|w| *w == "--watch" || *w == "--follow") {
        return None;
    }
    let w = strip_runners(&all);
    let prog = program_name(w.first()?);
    let args = &w[1..];
    let sub = first_positional(args).unwrap_or("");
    let has = |f: &str| args.contains(&f);
    let kind = match prog.as_str() {
        "git" => match git_subcommand(args) {
            "status" => GitStatus,
            "diff" | "show" => GitDiff,
            "log" => GitLog,
            "grep" => Search,
            "blame" | "reflog" | "ls-files" | "branch" | "tag" | "shortlog" => Generic,
            _ => return None,
        },
        "cargo" => match sub {
            "test" | "nextest" => Test,
            "build" | "check" | "clippy" | "doc" | "install" | "update" | "fetch" | "bench" => {
                Build
            }
            _ => return None,
        },
        "pytest" | "py.test" | "jest" | "vitest" | "mocha" | "ava" | "rspec" | "phpunit"
        | "tox" | "nox" | "ctest" => Test,
        "playwright" if sub == "test" => Test,
        "go" => match sub {
            "test" => Test,
            "build" | "vet" | "install" | "generate" | "mod" | "get" => Build,
            _ => return None,
        },
        "npm" | "pnpm" | "yarn" | "bun" => match sub {
            "test" | "t" => Test,
            "install" | "i" | "ci" | "add" | "update" | "upgrade" | "audit" | "outdated" => Build,
            "run" | "run-script" => {
                let script = args
                    .iter()
                    .skip_while(|a| **a != sub)
                    .skip(1)
                    .find(|a| !a.starts_with('-'))?;
                script_kind(script)?
            }
            "" if prog == "yarn" => Build,
            other if prog != "npm" => script_kind(other)?,
            _ => return None,
        },
        "pip" | "pip3" => match sub {
            "install" | "uninstall" | "download" | "wheel" => Build,
            _ => return None,
        },
        "uv" => match sub {
            "sync" | "add" | "lock" | "pip" | "remove" => Build,
            _ => return None,
        },
        "poetry" => match sub {
            "install" | "update" | "add" | "lock" => Build,
            _ => return None,
        },
        "gradle" | "gradlew" | "mvn" | "mvnw" | "bazel" | "bazelisk" | "swift" => {
            if sub == "test" {
                Test
            } else {
                Build
            }
        }
        "make" | "ninja" | "cmake" | "msbuild" | "tsc" | "eslint" | "ruff" | "mypy" | "pyright"
        | "flake8" | "pylint" | "golangci-lint" | "biome" | "stylelint" | "rubocop"
        | "shellcheck" | "hadolint" => Build,
        "dotnet" => match sub {
            "test" => Test,
            "build" | "restore" | "publish" | "pack" => Build,
            _ => return None,
        },
        "docker" | "podman" => match sub {
            "build" | "pull" | "push" => Build,
            "logs" if !has("-f") => Generic,
            "ps" | "images" | "inspect" => Generic,
            _ => return None,
        },
        "kubectl" | "oc" => match sub {
            "get" | "describe" | "logs" | "events" | "top" if !has("-f") && !has("-w") => Generic,
            _ => return None,
        },
        "journalctl" if !has("-f") => Generic,
        "dmesg" => Generic,
        "terraform" | "tofu" => match sub {
            "plan" | "validate" | "init" => Build,
            _ => return None,
        },
        "gh" => match sub {
            "api" | "run" | "pr" | "issue" | "search" => Generic,
            _ => return None,
        },
        "curl" | "http" | "xh" => Generic,
        "grep" | "egrep" | "fgrep" | "rg" | "ag" | "ack" => Search,
        "find" | "fd" | "fdfind" | "ls" | "tree" | "du" => Listing,
        _ => return None,
    };
    Some(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(cmd: &str) -> Option<Kind> {
        let w: Vec<&str> = cmd.split_whitespace().collect();
        classify(&w)
    }

    #[test]
    fn classification() {
        assert_eq!(c("git status"), Some(Kind::GitStatus));
        assert_eq!(c("git -C repo --no-pager log -n 5"), Some(Kind::GitLog));
        assert_eq!(c("git commit -m x"), None);
        assert_eq!(c("git push"), None);
        assert_eq!(c("cargo test --all"), Some(Kind::Test));
        assert_eq!(c("cargo +nightly test"), Some(Kind::Test));
        assert_eq!(c("python -m pytest -x"), Some(Kind::Test));
        assert_eq!(c("uv run pytest"), Some(Kind::Test));
        assert_eq!(c("npx jest"), Some(Kind::Test));
        assert_eq!(c("npm run test:unit"), Some(Kind::Test));
        assert_eq!(c("npm run dev"), None);
        assert_eq!(c("npm start"), None);
        assert_eq!(c("pnpm build"), Some(Kind::Build));
        assert_eq!(c("rg -n foo src"), Some(Kind::Search));
        assert_eq!(c("C:/tools/rg.exe foo"), Some(Kind::Search));
        assert_eq!(c("docker logs -f web"), None);
        assert_eq!(c("kubectl get pods -w"), None);
        assert_eq!(c("jest --watch"), None);
        assert_eq!(c("rm -rf target"), None);
        assert_eq!(c("cat file.txt"), None);
    }

    #[test]
    fn labels() {
        assert_eq!(label(&["git", "-C", "x", "diff"]), "git diff");
        assert_eq!(label(&["python", "-m", "pytest", "-x"]), "pytest");
        assert_eq!(label(&["npx", "-y", "jest", "src"]), "jest");
        assert_eq!(label(&["ls", "-la", "crates"]), "ls");
        assert_eq!(label(&["npm", "run", "build"]), "npm run");
    }
}
