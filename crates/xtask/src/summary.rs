//! `parity/summary.json` schema — spec §3 `audit --final` writes.
//! Fields: round, rows_by_status, tests_by_status, stubs_by_crate,
//! new_gaps, clean_streak, coverage_diff_line, coverage_diff_branch.

use crate::paths;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Summary {
    pub round: u32,
    pub rows_by_status: BTreeMap<String, u32>,
    pub tests_by_status: BTreeMap<String, u32>,
    pub stubs_by_crate: BTreeMap<String, u32>,
    pub new_gaps: u32,
    pub clean_streak: u32,
    pub coverage_diff_line: f64,
    pub coverage_diff_branch: f64,
}

pub fn load() -> Summary {
    let p = paths::summary_json();
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(s: &Summary) -> std::result::Result<(), std::io::Error> {
    let p = paths::summary_json();
    if let Some(p) = p.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(p, serde_json::to_string_pretty(s)?)
}
