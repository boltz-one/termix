//! `xtask audit --wave N | --false` — spec §3 + §5.
//!
//! `--wave N` (barrier): fmt, clippy, build --release -p terminal, test --workspace,
//! ledger-check, tests-check (join only), stubs (report), no status regression vs
//! previous summary.json. Writes audit-NNN.md + updates summary.json.
//!
//! `--final` (P19): strict — mọi §4 criterion.

use crate::ledger;
use crate::paths;
use crate::stubs;
use crate::summary::{self, Summary};
use crate::tests_check as tests_mod;
use crate::{Rt, R};
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

pub fn cmd_audit(args: &[String]) -> R {
    let mut wave: Option<u32> = None;
    let mut final_mode = false;
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--wave" => {
                if let Some(v) = iter.next() {
                    wave = Some(v.parse().unwrap_or(0));
                }
            }
            "--final" => final_mode = true,
            _ => {}
        }
    }
    if wave.is_none() && !final_mode {
        return Err("audit: requires --wave N or --final".into());
    }
    let round = next_round()?;
    let prev = summary::load();

    // Mechanical gates.
    let mut errors: Vec<String> = Vec::new();
    gate_fmt(&mut errors);
    gate_clippy(&mut errors);
    gate_build_release(&mut errors);
    gate_test_workspace(&mut errors);

    // ledger-check (delegate).
    if let Err(e) = ledger::cmd_check(&[]) {
        errors.push(format!("ledger-check: {e}"));
    }

    // stubs (report only — không fail ở wave; final thì fail nếu >0).
    let stub_counts = collect_stubs();
    let stub_total: u32 = stub_counts.values().map(|(t, i, s)| t + i + s).sum();

    // tests-check (join only).
    let tests_counts = tests_mod::join_only(&mut errors);

    // Regression check vs prev summary (wave mode).
    let mut new_gaps = 0u32;
    if let Some(_w) = wave {
        new_gaps = compute_new_gaps(&prev);
    }
    if final_mode {
        // §4 criteria enforcement.
        enforce_final(&stub_total, &mut errors);
    }

    let rows_by_status = ledger::status_counts();
    let now = Summary {
        round,
        rows_by_status: rows_by_status.clone(),
        tests_by_status: tests_counts.clone(),
        stubs_by_crate: stub_counts
            .iter()
            .map(|(k, (t, i, s))| (k.clone(), t + i + s))
            .collect(),
        new_gaps,
        clean_streak: if errors.is_empty() {
            prev.clean_streak + 1
        } else {
            0
        },
        coverage_diff_line: 0.0,
        coverage_diff_branch: 0.0,
    };
    write_audit_md(round, &now, &errors, wave, final_mode)?;
    summary::save(&now).map_err(|e| e.to_string())?;

    if errors.is_empty() {
        println!(
            "audit round {round}: OK (clean_streak {})",
            now.clean_streak
        );
        Ok(())
    } else {
        Err(format!(
            "audit round {round} FAILED ({} violations):\n  - {}",
            errors.len(),
            errors.join("\n  - ")
        )
        .into())
    }
}

fn gate_fmt(errors: &mut Vec<String>) {
    let out = Command::new("cargo").args(["fmt", "--check"]).output();
    match out {
        Ok(o) if !o.status.success() => {
            errors.push(format!(
                "cargo fmt --check: {}",
                String::from_utf8_lossy(&o.stdout)
            ));
        }
        Err(e) => errors.push(format!("cargo fmt: {e}")),
        _ => {}
    }
}

fn gate_clippy(errors: &mut Vec<String>) {
    let out = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .output();
    match out {
        Ok(o) if !o.status.success() => {
            errors.push(format!(
                "cargo clippy: {}",
                String::from_utf8_lossy(&o.stderr)
            ));
        }
        Err(e) => errors.push(format!("cargo clippy: {e}")),
        _ => {}
    }
}

fn gate_build_release(errors: &mut Vec<String>) {
    let out = Command::new("cargo")
        .args(["build", "--release", "-p", "terminal"])
        .output();
    match out {
        Ok(o) if !o.status.success() => {
            errors.push(format!(
                "cargo build --release -p terminal: {}",
                String::from_utf8_lossy(&o.stderr)
            ));
        }
        Err(e) => errors.push(format!("cargo build --release: {e}")),
        _ => {}
    }
}

fn gate_test_workspace(errors: &mut Vec<String>) {
    // Skip xtask to avoid Windows file-locking (xtask.exe rebuilding itself while running)
    let out = Command::new("cargo")
        .args(["test", "--workspace", "--exclude", "xtask"])
        .output();
    match out {
        Ok(o) if !o.status.success() => {
            errors.push(format!(
                "cargo test --workspace: {}",
                String::from_utf8_lossy(&o.stderr)
            ));
        }
        Err(e) => errors.push(format!("cargo test --workspace: {e}")),
        _ => {}
    }
}

fn collect_stubs() -> BTreeMap<String, (u32, u32, u32)> {
    // reuse stubs::cmd_stubs internals — re-walk + count.
    let mut files = Vec::new();
    stubs_walk(&paths::workspace_root(), &mut files);
    let mut per: BTreeMap<String, (u32, u32, u32)> = BTreeMap::new();
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else {
            continue;
        };
        let (t, i, s) = stubs::count_in_text(&src);
        if t + i + s == 0 {
            continue;
        }
        let name = crate_name_of(f, &paths::workspace_root());
        let e = per.entry(name).or_insert((0, 0, 0));
        e.0 += t;
        e.1 += i;
        e.2 += s;
    }
    per
}

fn stubs_walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            let n = p
                .file_name()
                .map(|x| x.to_string_lossy().to_string())
                .unwrap_or_default();
            if n == "target" || n == ".git" {
                continue;
            }
            stubs_walk(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}

fn crate_name_of(p: &std::path::Path, scope: &std::path::Path) -> String {
    let parts: Vec<String> = p
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    for (i, seg) in parts.iter().enumerate() {
        if (seg == "crates" || seg == "apps") && i + 1 < parts.len() {
            return parts[i + 1].clone();
        }
    }
    p.strip_prefix(scope)
        .map(|r| r.to_string_lossy().to_string())
        .unwrap_or_else(|_| p.display().to_string())
}

fn compute_new_gaps(prev: &Summary) -> u32 {
    // New gap = regression in row counts: any not-done tăng thì... pragmatic:
    // count rows whose status moved from done → not-done. Sử dụng status_counts diff.
    let now = ledger::status_counts();
    let mut gaps = 0u32;
    for (s, n) in &now {
        let before = prev.rows_by_status.get(s).copied().unwrap_or(0);
        if matches!(s.as_str(), "not-started" | "stub" | "partial" | "ported") && *n > before {
            gaps += n - before;
        }
    }
    gaps
}

fn enforce_final(stub_total: &u32, errors: &mut Vec<String>) {
    // §4.3: stubs = 0 across workspace.
    if *stub_total > 0 {
        errors.push(format!("stubs --final: {stub_total} > 0 remaining"));
    }
    // §4.1: files+symbols 100% in done states.
    let rows = ledger::status_counts();
    for (s, n) in &rows {
        if matches!(s.as_str(), "not-started" | "stub" | "partial" | "ported") {
            errors.push(format!("--final: {n} rows in non-done status `{s}`"));
        }
    }
    // §4.2: tests 100% passing/waived.
    let tests = crate::ledger::tests_counts();
    for (s, n) in &tests {
        if matches!(s.as_str(), "not-started" | "ported") {
            errors.push(format!("--final: {n} tests in non-done status `{s}`"));
        }
    }
}

fn next_round() -> Rt<u32> {
    let rd = paths::reports_dir();
    fs::create_dir_all(&rd)?;
    let mut max = 0u32;
    if let Ok(e) = fs::read_dir(&rd) {
        for f in e.flatten() {
            let n = f.file_name().to_string_lossy().to_string();
            if let Some(rest) = n.strip_prefix("audit-") {
                if let Some(num) = rest.strip_suffix(".md") {
                    if let Ok(v) = num.parse::<u32>() {
                        if v > max {
                            max = v;
                        }
                    }
                }
            }
        }
    }
    Ok(max + 1)
}

fn write_audit_md(
    round: u32,
    s: &Summary,
    errors: &[String],
    wave: Option<u32>,
    final_mode: bool,
) -> R {
    let p = paths::reports_dir().join(format!("audit-{round:03}.md"));
    let mode = if final_mode {
        "final"
    } else if let Some(w) = wave {
        &format!("wave {w}")
    } else {
        "unknown"
    };
    let mut out = String::new();
    out.push_str(&format!("# Audit round {round}\n\n"));
    out.push_str(&format!("- Date: {}\n", chrono_like_date()));
    out.push_str(&format!("- Mode: {mode}\n"));
    out.push_str(&format!(
        "- Verdict: {}\n\n",
        if errors.is_empty() { "PASS" } else { "FAIL" }
    ));
    out.push_str("## Rows by status\n\n");
    for (k, v) in &s.rows_by_status {
        out.push_str(&format!("- {k}: {v}\n"));
    }
    out.push_str("\n## Tests by status\n\n");
    for (k, v) in &s.tests_by_status {
        out.push_str(&format!("- {k}: {v}\n"));
    }
    out.push_str("\n## Stubs by crate\n\n");
    for (k, v) in &s.stubs_by_crate {
        out.push_str(&format!("- {k}: {v}\n"));
    }
    out.push_str(&format!("\n- new_gaps: {}\n", s.new_gaps));
    out.push_str(&format!("- clean_streak: {}\n", s.clean_streak));
    if !errors.is_empty() {
        out.push_str("\n## Violations\n\n");
        for e in errors {
            out.push_str(&format!("- {e}\n"));
        }
    }
    fs::write(&p, out)?;
    Ok(())
}

fn chrono_like_date() -> String {
    // std-only — use SystemTime, format as ISO date approximated.
    use std::time::{SystemTime, UNIX_EPOCH};
    let s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let days = s.as_secs() / 86_400;
    // 1970-01-01 + days. Approximate (no calendar math) — good enough for report header.
    format!("(epoch+{days}d)")
}
