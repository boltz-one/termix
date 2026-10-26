//! `xtask loc-ratio` — spec §3.
//!
//! Rust LOC / C++ LOC per module — warning only (never exits non-zero).
//! Ratio < 0.25 trên `verified` row's module listed cho reviewer attention.

use crate::ledger::{read_csv, FilesRow};
use crate::paths;
use crate::R;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn cmd_loc_ratio(_args: &[String]) -> R {
    let mut cpp_loc: BTreeMap<String, u32> = BTreeMap::new(); // module → loc
    let mut rust_loc: BTreeMap<String, u32> = BTreeMap::new();
    let mut verified_modules: BTreeSet = BTreeSet::new();

    for s in crate::ledger::shard_dirs() {
        let fp = s.join("files.csv");
        if !fp.exists() {
            continue;
        }
        let rows: Vec<FilesRow> = read_csv(&fp).unwrap_or_default();
        for r in &rows {
            let m = r.module.clone();
            let _cpp = paths::windows_src().join(&r.wt_path).with_extension("");
            // wt_loc field có thể 0 — re-count từ file if exists.
            let cl = if r.wt_loc > 0 {
                r.wt_loc as usize
            } else {
                count_lines(&paths::windows_src().join(&r.wt_path))
            };
            *cpp_loc.entry(m.clone()).or_insert(0) += cl as u32;
            if r.status == "verified" {
                verified_modules.insert(m.clone());
            }
            if !r.rust_path.is_empty() {
                let rp = paths::workspace_root().join(&r.rust_path);
                let rl = count_lines(&rp);
                *rust_loc.entry(m.clone()).or_insert(0) += rl as u32;
            }
        }
    }
    let mut warnings = Vec::new();
    for (m, cl) in &cpp_loc {
        let rl = rust_loc.get(m).copied().unwrap_or(0);
        let ratio = if *cl == 0 {
            0.0
        } else {
            rl as f64 / *cl as f64
        };
        if ratio < 0.25 && verified_modules.contains(m) {
            warnings.push(format!(
                "loc-ratio: module `{m}` ratio {ratio:.2} < 0.25 (rust={rl}, cpp={cl}) — verified; reviewer attention"
            ));
        }
    }
    if warnings.is_empty() {
        println!("loc-ratio: no warnings");
    } else {
        println!("loc-ratio: {} warnings (never fails):", warnings.len());
        for w in &warnings {
            println!("  - {w}");
        }
    }
    Ok(())
}

fn count_lines(p: &Path) -> usize {
    fs::read_to_string(p)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

// Local BTreeSet alias — avoid extra import line noise.
type BTreeSet = std::collections::BTreeSet<String>;

#[allow(dead_code)]
fn _suppress_unused(_: PathBuf) {}
