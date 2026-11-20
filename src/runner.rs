//! Run a command with stdout and stderr merged into one pipe, so lines keep their
//! real interleaving, and report the exit code faithfully.

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

pub struct Captured {
    pub text: String,
    pub code: i32,
}

pub fn run(args: &[String]) -> io::Result<Captured> {
    let (prog, rest) = args
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no command given"))?;
    let (mut reader, writer) = io::pipe()?;
    let writer2 = writer.try_clone()?;
    let program = resolve(prog);
    let mut cmd = Command::new(&program);
    add_args(&mut cmd, &program, rest);
    cmd.stdin(Stdio::inherit()).stdout(writer).stderr(writer2);
    let mut child = cmd.spawn()?;
    // The Command still holds the write ends; drop it so the read sees EOF.
    drop(cmd);
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    let status = child.wait()?;
    Ok(Captured {
        text: String::from_utf8_lossy(&buf).into_owned(),
        code: exit_code(status),
    })
}

/// On Windows, MSYS/Cygwin programs (Git Bash's `find`, `grep`, `ls`, ...) glob-expand
/// unquoted arguments themselves, so `find . -name *.py` would see the expanded file
/// list. The shell already did all expansion, so every argument is passed quoted.
/// Batch files keep Rust's own escaping, which guards against cmd.exe injection.
#[cfg(windows)]
fn add_args(cmd: &mut Command, program: &std::path::Path, args: &[String]) {
    use std::os::windows::process::CommandExt;
    let is_batch = program
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("bat") || e.eq_ignore_ascii_case("cmd"));
    if is_batch {
        cmd.args(args);
    } else {
        for a in args {
            cmd.raw_arg(quote_windows_arg(a));
        }
    }
}

#[cfg(not(windows))]
fn add_args(cmd: &mut Command, _program: &std::path::Path, args: &[String]) {
    cmd.args(args);
}

/// Quote one argument by the MSVC C runtime rules (also understood by MSYS).
#[cfg_attr(not(windows), allow(dead_code))]
fn quote_windows_arg(arg: &str) -> String {
    let mut s = String::with_capacity(arg.len() + 2);
    s.push('"');
    let mut backslashes = 0;
    for c in arg.chars() {
        if c == '\\' {
            backslashes += 1;
            continue;
        }
        let n = if c == '"' {
            backslashes * 2 + 1
        } else {
            backslashes
        };
        s.extend(std::iter::repeat_n('\\', n));
        backslashes = 0;
        s.push(c);
    }
    s.extend(std::iter::repeat_n('\\', backslashes * 2));
    s.push('"');
    s
}

fn exit_code(status: ExitStatus) -> i32 {
    if let Some(c) = status.code() {
        return c;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return 128 + sig;
        }
    }
    1
}

/// Windows needs help finding `npm.cmd`, `yarn.cmd` and similar: Rust only appends `.exe`.
#[cfg(windows)]
fn resolve(prog: &str) -> PathBuf {
    use std::path::Path;
    let p = Path::new(prog);
    if p.parent().is_some_and(|d| !d.as_os_str().is_empty()) {
        return p.to_path_buf();
    }
    let exts: Vec<String> = std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
        .split(';')
        .filter(|e| !e.is_empty())
        .map(|e| e.to_ascii_lowercase())
        .collect();
    let has_ext = p.extension().is_some();
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            if has_ext {
                let c = dir.join(prog);
                if c.is_file() {
                    return c;
                }
            }
            for ext in &exts {
                let c = dir.join(format!("{prog}{ext}"));
                if c.is_file() {
                    return c;
                }
            }
        }
    }
    p.to_path_buf()
}

#[cfg(not(windows))]
fn resolve(prog: &str) -> PathBuf {
    PathBuf::from(prog)
}

#[cfg(test)]
mod tests {
    use super::quote_windows_arg;

    #[test]
    fn windows_quoting() {
        assert_eq!(quote_windows_arg("*.py"), r#""*.py""#);
        assert_eq!(quote_windows_arg(r"C:\dir\"), r#""C:\dir\\""#);
        assert_eq!(quote_windows_arg(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(quote_windows_arg(r#"a\"b"#), r#""a\\\"b""#);
        assert_eq!(quote_windows_arg(""), r#""""#);
    }
}
