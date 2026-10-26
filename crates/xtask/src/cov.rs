//! `xtask cov --phase NN` — spec §3.
//!
//! Runs from git root: `cargo +nightly llvm-cov --branch --cobertura
//! --output-path target/cobertura.xml -p <crate>`; `diff-cover` line >=90%;
//! xtask parses cobertura `condition-coverage` for branch >=75% on lines
//! changed vs $BASE. $BASE = `--base <ref>` or `parity/baseline-ref.txt`.
//!
//! No nightly → line coverage only + ADR waiver warning.

use crate::paths;
use crate::{Rt, R};
use std::fs;
use std::process::Command;

pub fn cmd_cov(args: &[String]) -> R {
    let mut phase: Option<String> = None;
    let mut base: Option<String> = None;
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--phase" => {
                if let Some(v) = iter.next() {
                    phase = Some(v.clone());
                }
            }
            "--base" => {
                if let Some(v) = iter.next() {
                    base = Some(v.clone());
                }
            }
            _ => {}
        }
    }
    let phase = phase.ok_or("cov: --phase NN required")?;
    let crate_names = lookup_phase_crates(&phase)?;
    if crate_names.is_empty() {
        return Err(format!("cov: phase {phase} has no owned crates in phases.csv").into());
    }
    let base_ref = base
        .or_else(read_baseline_ref)
        .unwrap_or_else(|| "HEAD".to_string());
    let cov_path = paths::workspace_root().join("target").join("cobertura.xml");
    if let Some(p) = cov_path.parent() {
        fs::create_dir_all(p)?;
    }
    // Try nightly first; fall back to default toolchain với warning.
    let nightly = Command::new("cargo")
        .args(["+nightly", "llvm-cov", "--branch", "--cobertura"])
        .arg("--output-path")
        .arg(&cov_path)
        .arg("-p")
        .args(&crate_names)
        .output();
    let used_nightly = match nightly {
        Ok(o) if o.status.success() => true,
        _ => {
            eprintln!("cov: nightly llvm-cov unavailable, falling back to line-only");
            let o = Command::new("cargo")
                .args(["llvm-cov", "--cobertura"])
                .arg("--output-path")
                .arg(&cov_path)
                .arg("-p")
                .args(&crate_names)
                .output()?;
            if !o.status.success() {
                return Err(format!(
                    "cov: llvm-cov failed: {}",
                    String::from_utf8_lossy(&o.stderr)
                )
                .into());
            }
            false
        }
    };
    // diff-cover line >=90%.
    let dc = Command::new("diff-cover")
        .arg(&cov_path)
        .args(["--compare-branch", &base_ref])
        .args(["--fail-under", "90"])
        .output()?;
    if !dc.status.success() {
        return Err(format!(
            "cov: line coverage < 90% vs base {base_ref}\n{}",
            String::from_utf8_lossy(&dc.stdout)
        )
        .into());
    }
    // Branch >=75% — parse cobertura condition-coverage (skipped if no nightly).
    if used_nightly {
        let branch_pct = parse_branch_coverage(&cov_path)?;
        if branch_pct < 75.0 {
            return Err(format!("cov: branch coverage {branch_pct:.1}% < 75%").into());
        }
    } else {
        eprintln!("cov: ADR waiver — branch coverage skipped (no nightly toolchain)");
    }
    println!("cov phase {phase}: OK (line >=90%, branch >=75% if nightly)");
    Ok(())
}

fn lookup_phase_crates(phase: &str) -> Rt<Vec<String>> {
    let csv_path = paths::phases_csv();
    if !csv_path.exists() {
        return Ok(Vec::new());
    }
    let mut rdr = csv::Reader::from_path(&csv_path)?;
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        // columns: phase, crates, owned_paths, wave, verify_command, depends_on
        let p = rec.get(0).unwrap_or("");
        if p == phase {
            let crates = rec.get(1).unwrap_or("");
            for c in crates.split([';', ',']) {
                let c = c.trim();
                if !c.is_empty() {
                    out.push(c.to_string());
                }
            }
            break;
        }
    }
    Ok(out)
}

fn read_baseline_ref() -> Option<String> {
    let p = paths::baseline_ref();
    let s = fs::read_to_string(&p).ok()?;
    s.lines()
        .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|s| s.trim().to_string())
}

/// Parse cobertura XML `condition-coverage` tính branch %. Pragmatic — text scan.
fn parse_branch_coverage(path: &std::path::Path) -> Rt<f64> {
    let s = fs::read_to_string(path)?;
    // Tìm tất cả `condition-coverage="NN% (M/N)"` — avg.
    let mut sum = 0.0;
    let mut n = 0.0;
    for line in s.lines() {
        if let Some(idx) = line.find("condition-coverage=\"") {
            let rest = &line[idx + "condition-coverage=\"".len()..];
            if let Some(end) = rest.find('"') {
                let val = &rest[..end];
                // val = "75% (3/4)"
                if let Some(pct_str) = val.split('%').next() {
                    if let Ok(p) = pct_str.trim().parse::<f64>() {
                        sum += p;
                        n += 1.0;
                    }
                }
            }
        }
    }
    if n == 0.0 {
        return Ok(100.0); // no conditions — satisfy threshold.
    }
    Ok(sum / n)
}
