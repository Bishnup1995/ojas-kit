//! Per-run savings log (`~/.tsave/stats.jsonl`) and the `tsave gain` report.
//! Token counts here are estimates (bytes / 4). `tsave audit` reads real usage
//! from Claude Code transcripts instead.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;

use crate::paths;

#[derive(Serialize, Deserialize)]
struct Rec {
    ts: u64,
    cmd: String,
    kind: String,
    raw: usize,
    out: usize,
}

pub fn record(cmd: &str, kind: &str, raw: usize, out: usize) {
    let home = paths::tsave_home();
    if fs::create_dir_all(&home).is_err() {
        return;
    }
    let rec = Rec {
        ts: paths::now_secs(),
        cmd: cmd.to_string(),
        kind: kind.to_string(),
        raw,
        out,
    };
    if let (Ok(line), Ok(mut f)) = (
        serde_json::to_string(&rec),
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(home.join("stats.jsonl")),
    ) {
        let _ = writeln!(f, "{line}");
    }
}

pub fn tok(bytes: usize) -> usize {
    bytes.div_ceil(4)
}

pub fn fmt_num(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[derive(Default)]
struct Agg {
    runs: usize,
    compressed: usize,
    raw: usize,
    out: usize,
}

impl Agg {
    fn add(&mut self, r: &Rec) {
        self.runs += 1;
        if r.out < r.raw {
            self.compressed += 1;
        }
        self.raw += r.raw;
        self.out += r.out;
    }
    fn pct(&self) -> f64 {
        if self.raw == 0 {
            0.0
        } else {
            100.0 * (self.raw.saturating_sub(self.out)) as f64 / self.raw as f64
        }
    }
}

pub fn gain(days: Option<u64>) -> String {
    let path = paths::tsave_home().join("stats.jsonl");
    let text = fs::read_to_string(&path).unwrap_or_default();
    let cutoff = days.map(|d| paths::now_secs().saturating_sub(d * 86_400));
    let recs: Vec<Rec> = text
        .lines()
        .filter_map(|l| serde_json::from_str::<Rec>(l).ok())
        .filter(|r| cutoff.is_none_or(|c| r.ts >= c))
        .collect();
    if recs.is_empty() {
        return "No tsave runs recorded yet. Install the hook with `tsave init`, or run commands as `tsave run -- <cmd>`.".into();
    }
    let mut total = Agg::default();
    let mut by_kind: BTreeMap<String, Agg> = BTreeMap::new();
    let mut by_cmd: BTreeMap<String, Agg> = BTreeMap::new();
    for r in &recs {
        total.add(r);
        by_kind.entry(r.kind.clone()).or_default().add(r);
        by_cmd.entry(r.cmd.clone()).or_default().add(r);
    }
    let mut s = String::new();
    let span = match days {
        Some(d) => format!("last {d} days"),
        None => "all time".into(),
    };
    s.push_str(&format!(
        "tsave savings ({span}; estimate: 1 token ≈ 4 bytes)\n\n"
    ));
    s.push_str(&format!(
        "runs {}   compressed {}   raw ≈{} tok   sent ≈{} tok   saved ≈{} tok ({:.0}%)\n\n",
        fmt_num(total.runs),
        fmt_num(total.compressed),
        fmt_num(tok(total.raw)),
        fmt_num(tok(total.out)),
        fmt_num(tok(total.raw.saturating_sub(total.out))),
        total.pct()
    ));
    s.push_str(&format!(
        "{:<12} {:>6} {:>10} {:>12} {:>12} {:>6}\n",
        "filter", "runs", "compressed", "raw tok", "sent tok", "saved"
    ));
    for (k, a) in &by_kind {
        s.push_str(&format!(
            "{:<12} {:>6} {:>10} {:>12} {:>12} {:>5.0}%\n",
            k,
            a.runs,
            a.compressed,
            fmt_num(tok(a.raw)),
            fmt_num(tok(a.out)),
            a.pct()
        ));
    }
    let mut cmds: Vec<(&String, &Agg)> = by_cmd.iter().collect();
    cmds.sort_by_key(|(_, a)| std::cmp::Reverse(a.raw.saturating_sub(a.out)));
    s.push_str("\ntop commands by tokens saved\n");
    for (c, a) in cmds.iter().take(10) {
        s.push_str(&format!(
            "  {:<28} {:>5} runs  saved ≈{} tok ({:.0}%)\n",
            c,
            a.runs,
            fmt_num(tok(a.raw.saturating_sub(a.out))),
            a.pct()
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formatting() {
        assert_eq!(fmt_num(0), "0");
        assert_eq!(fmt_num(999), "999");
        assert_eq!(fmt_num(1234567), "1,234,567");
    }
}
