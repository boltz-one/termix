//! Path resolution — git root `terminal/` (parent của `terminal_gpui/`).
//! `windows/` read-only upstream; `terminal_gpui/` = rust workspace; `parity/`,
//! `reports/`, `docs/` ở repo-level `terminal_gpui/`. Spec report 02 §3.

use std::path::{Path, PathBuf};

/// Git root = parent của `terminal_gpui/`. Spec: "xtask runs from the git root `terminal/`".
pub fn git_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = terminal_gpui/crates/xtask → ../../ = terminal_gpui/, ../ = terminal/
    let manifest = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest)
        .parent() // crates/
        .and_then(|p| p.parent()) // terminal_gpui/
        .and_then(|p| p.parent()) // terminal/
        .unwrap_or_else(|| Path::new(".."))
        .to_path_buf()
}

/// `terminal_gpui/` — Rust workspace root.
pub fn workspace_root() -> PathBuf {
    git_root().join("terminal_gpui")
}

/// `windows/src` — upstream read-only.
pub fn windows_src() -> PathBuf {
    git_root().join("windows").join("src")
}

pub fn parity_dir() -> PathBuf {
    workspace_root().join("parity")
}

pub fn reports_dir() -> PathBuf {
    workspace_root().join("reports")
}

pub fn ledger_dir() -> PathBuf {
    parity_dir().join("ledger")
}

pub fn exclusions_csv() -> PathBuf {
    parity_dir().join("exclusions.csv")
}

pub fn phases_csv() -> PathBuf {
    parity_dir().join("phases.csv")
}

pub fn e2e_csv() -> PathBuf {
    parity_dir().join("e2e.csv")
}

pub fn baseline_ref() -> PathBuf {
    parity_dir().join("baseline-ref.txt")
}

pub fn summary_json() -> PathBuf {
    parity_dir().join("summary.json")
}

pub fn merged_dir() -> PathBuf {
    parity_dir().join("merged")
}

pub fn golden_dir() -> PathBuf {
    parity_dir().join("golden")
}
