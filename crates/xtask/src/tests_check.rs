//! `xtask tests-check` — spec §3.
//!
//! Join tests.csv ↔ `cargo test --workspace -- --list`. Broken join luôn fail;
//! in-scope row `not-started` chỉ fail ở `--final`.

use crate::ledger::shard_dirs;
use crate::ledger::{read_csv, TestsRow};
use crate::R;
use std::collections::BTreeMap;
use std::process::Command;

/// Join-only — không fail trên `not-started` (chỉ `--final`). Trả status counts.
pub fn join_only(errors: &mut Vec<String>) -> BTreeMap<String, u32> {
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut all: Vec<TestsRow> = Vec::new();
    for s in shard_dirs() {
        let tp = s.join("tests.csv");
        if tp.exists() {
            if let Ok(rows) = read_csv::<TestsRow>(&tp) {
                for r in &rows {
                    *counts.entry(r.status.clone()).or_insert(0) += 1;
                }
                all.extend(rows);
            }
        }
    }
    if all.is_empty() {
        return counts;
    }
    // `cargo test --workspace -- --list` outputs test names. Best-effort join.
    let out = Command::new("cargo")
        .args(["test", "--workspace", "--", "--list"])
        .output();
    let listed: std::collections::BTreeSet<String> = match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().trim_end_matches(": test").to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        Err(_) => std::collections::BTreeSet::new(),
    };
    for r in &all {
        if r.status == "passing" && !r.rust_test.is_empty()
            && !listed.iter().any(|n| n.contains(&r.rust_test))
        {
            errors.push(format!(
                "tests-check: passing row {} rust_test `{}` not in `cargo test --list`",
                r.id, r.rust_test
            ));
        }
    }
    counts
}

pub fn cmd_tests_check(args: &[String]) -> R {
    let final_mode = args.iter().any(|a| a == "--final");
    let mut errors = Vec::new();
    let counts = join_only(&mut errors);
    if final_mode {
        for (s, n) in &counts {
            if matches!(s.as_str(), "not-started" | "ported") {
                errors.push(format!("tests-check --final: {n} tests in `{s}`"));
            }
        }
    }
    if errors.is_empty() {
        println!("tests-check: OK");
        for (k, v) in &counts {
            println!("  {k}: {v}");
        }
        Ok(())
    } else {
        Err(format!(
            "tests-check FAILED ({}):\n  - {}",
            errors.len(),
            errors.join("\n  - ")
        )
        .into())
    }
}
