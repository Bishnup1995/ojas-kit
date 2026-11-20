//! Builds, installs and linters: drop progress noise, fold deprecation spam.

use regex::{Regex, RegexSet};
use std::sync::LazyLock;

/// Progress and bookkeeping lines that carry no information for an agent.
pub static NOISE: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new([
        // cargo
        r"^\s*(?:Compiling|Checking|Downloaded|Downloading|Fresh|Blocking|Locking|Updating|Adding|Removing|Unpacking|Packaging|Verifying|Documenting|Archiving|Installing|Fetching|Resolving|Building)\s+\S",
        // pip / uv
        r"^\s*(?:Collecting|Using cached|Requirement already satisfied|Obtaining|Building wheel|Created wheel|Stored in directory|Preparing metadata|Getting requirements|Attempting uninstall|Found existing installation|Uninstalling|Successfully uninstalled)\b",
        r"^\s*(?:Resolved|Prepared|Installed|Audited|Uninstalled) \d+ packages? in ",
        // docker pull / buildkit
        r"^[0-9a-f]{12}: (?:Pulling|Waiting|Downloading|Verifying|Download complete|Pull complete|Extracting|Already exists|Pushed|Pushing|Preparing|Layer already exists)",
        r"^#\d+ (?:\[internal\]|CACHED|sha256:|extracting|resolve|transferring|naming|exporting|writing|DONE)",
        // generic progress bars and spinners
        r"^\s*\d{1,3}%\s*[|\[]",
        r"^\s*[\-\\|/⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]\s*$",
        r"(?i)^\s*(?:progress|downloading|uploading):?\s.*\d+%",
        // gradle / maven
        r"^\s*(?:> Task \S+(?: UP-TO-DATE| NO-SOURCE| FROM-CACHE| SKIPPED)?|Download(?:ing|ed) from \S+:)\s*",
    ])
    .unwrap()
});

static DEPRECATED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*npm (?:warn|WARN) deprecated").unwrap());

pub fn compress(text: &str) -> String {
    let mut kept = Vec::new();
    let mut noise = 0usize;
    let mut deprecated = 0usize;
    let mut first_dep: Option<&str> = None;
    for line in text.lines() {
        if DEPRECATED.is_match(line) {
            deprecated += 1;
            first_dep.get_or_insert(line);
        } else if NOISE.is_match(line) {
            noise += 1;
        } else {
            kept.push(line.to_string());
        }
    }
    let mut out = Vec::new();
    if noise > 0 {
        out.push(format!("[ojas: {noise} progress lines hidden]"));
    }
    if let Some(d) = first_dep {
        if deprecated > 1 {
            out.push(format!(
                "{d}  [+{} more deprecation warnings]",
                deprecated - 1
            ));
        } else {
            out.push(d.to_string());
        }
    }
    out.extend(kept);
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_cargo_progress() {
        let raw = "   Compiling serde v1.0.0\n   Compiling regex v1.11.0\nwarning: unused variable: `x`\n    Finished `dev` profile";
        assert_eq!(
            compress(raw),
            "[ojas: 2 progress lines hidden]\nwarning: unused variable: `x`\n    Finished `dev` profile"
        );
    }
}
