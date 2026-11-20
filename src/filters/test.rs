//! Test runners: hide passing tests and build progress, keep failures and summaries.
//! Covers cargo, pytest, jest/vitest/mocha, go test and similar output.

use regex::RegexSet;
use std::sync::LazyLock;

use super::build::NOISE;

static PASS: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new([
        r"^test \S.* \.\.\. ok$",                   // cargo
        r"^running \d+ tests?$",                    // cargo
        r"^\s*Running (?:unittests |tests[/\\]|`)", // cargo
        r"^\s*Doc-tests \S+$",                      // cargo
        r"^\s*(?:✓|✔|√)\s",                         // jest / vitest / mocha
        r"^\s*PASS\s+\S",                           // jest file summary
        r"^=== (?:RUN|PAUSE|CONT|NAME)\s",          // go
        r"^\s*--- PASS: ",                          // go
        r"^ok\s+\S+\s+(?:\(cached\)|[\d.]+s)",      // go package ok
        r"^\S+::\S+ PASSED",                        // pytest -v
        r"^\S+\.py \.+\s*(?:\[\s*\d+%\])?$",        // pytest progress, all dots
        r"^\s*\.+\s*(?:\[\s*\d+%\])?$",             // bare dot lines
        r"^\s*\[\s*PASSED\s*\]|^\s*\[\s*OK\s*\]",   // gtest
    ])
    .unwrap()
});

pub fn compress(text: &str) -> String {
    let mut kept = Vec::new();
    let (mut passed, mut noise) = (0usize, 0usize);
    for line in text.lines() {
        if PASS.is_match(line) {
            passed += 1;
        } else if NOISE.is_match(line) {
            noise += 1;
        } else {
            kept.push(line);
        }
    }
    let mut out = Vec::new();
    if passed + noise > 0 {
        out.push(format!(
            "[ojas: {passed} passing-test lines and {noise} progress lines hidden]"
        ));
    }
    out.extend(kept.iter().map(|s| s.to_string()));
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_failures_survive() {
        let raw = "   Compiling app v0.1.0\nrunning 3 tests\ntest a ... ok\ntest b ... ok\ntest c ... FAILED\n\nfailures:\n\n---- c stdout ----\nthread 'c' panicked at src/lib.rs:9:5\n\ntest result: FAILED. 2 passed; 1 failed";
        let out = compress(raw);
        assert!(out.starts_with("[ojas: 3 passing-test lines and 1 progress lines hidden]"));
        assert!(out.contains("test c ... FAILED"));
        assert!(out.contains("panicked at src/lib.rs:9:5"));
        assert!(out.contains("test result: FAILED. 2 passed; 1 failed"));
        assert!(!out.contains("test a ... ok"));
    }

    #[test]
    fn jest_and_go() {
        let raw = "PASS src/a.test.ts\n  ✓ adds (2 ms)\n  ✕ subtracts (3 ms)\n=== RUN   TestX\n--- PASS: TestX (0.00s)\n--- FAIL: TestY (0.01s)\nok  \tpkg/a\t0.012s";
        let out = compress(raw);
        assert!(out.contains("✕ subtracts"));
        assert!(out.contains("--- FAIL: TestY"));
        assert!(!out.contains("✓ adds"));
        assert!(!out.contains("--- PASS"));
    }
}
