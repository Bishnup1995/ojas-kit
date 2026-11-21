//! User configuration, read from `~/.ojas/config.json`. Every field is optional.

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Master switch for compression. When false, `ojas run` passes output through.
    pub enabled: bool,
    /// Response style injected at session start: `off` (default), `lite` or `full`.
    /// Off by default: in benchmarks the style rules added turns and cost and did
    /// not shorten answers.
    pub style: String,
    /// Output at or below both of these limits is never compressed.
    pub min_lines: usize,
    pub min_bytes: usize,
    /// Line budget for most filters.
    pub max_lines: usize,
    /// Larger line budget for diffs, which are high-value.
    pub diff_max_lines: usize,
    /// Stored originals older than this are deleted.
    pub store_days: u64,
    /// Add a repo map (most-referenced definitions) to the session context.
    pub repo_map: bool,
    /// Token budget for the repo map.
    pub repo_map_budget: usize,
    /// `cat` of a source file longer than this is rewritten to `ojas read` (outline).
    pub read_min_lines: usize,
    /// A whole-file Read (no offset/limit) of a source file longer than this shows
    /// an outline instead of the full text. 0 turns this off. Higher than
    /// `read_min_lines` because the Read tool is the agent's main way to look at
    /// code before editing it, and each outline can cost an extra ranged read.
    pub read_tool_min_lines: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            style: "off".into(),
            min_lines: 40,
            min_bytes: 3000,
            max_lines: 150,
            diff_max_lines: 400,
            store_days: 7,
            repo_map: false,
            repo_map_budget: 1500,
            read_min_lines: 300,
            read_tool_min_lines: 500,
        }
    }
}

pub fn load() -> Config {
    let path = paths::ojas_home().join("config.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
