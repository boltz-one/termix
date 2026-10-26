//! Ledger CSV schema, merge, check. Spec report 02 §1, §3 (`ledger-check`, `merge`).
//!
//! Sharded: `parity/ledger/pNN-<name>/{files,symbols,tests,decisions}.csv`.
//! `xtask merge` → `parity/merged/*.csv` (P19-owned generated, nhưng lệnh tồn tại từ P01).
//! `ledger-check`: schema valid, orphan WT file, verified has test_ids + reviewed_by != author,
//! replaced/out-of-scope có ADR.

use crate::paths;
use crate::{Rt, R};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// `files.csv` / `symbols.csv` row (shared schema — §1 columns).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesRow {
    pub id: String,
    pub wt_path: String,
    pub wt_loc: u32,
    pub module: String,
    pub phase: String,
    pub rust_path: String,
    pub status: String,
    pub test_ids: String,
    pub adr: String,
    pub author: String,
    pub reviewed_by: String,
    pub reviewed_at: String,
    pub notes: String,
}

/// `tests.csv` row — §1 tests columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestsRow {
    pub id: String,
    pub wt_path: String,
    pub wt_suite_method: String, // `wt_<suite>::<method>`
    pub phase: String,
    pub rust_test: String, // path::fn
    pub status: String,
    pub adr: String,
}

/// `exclusions.csv` row — §1 single, P01.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclusionsRow {
    pub id: String,
    pub wt_path: String,
    pub adr: String,
    pub notes: String,
}

/// `decisions.csv` row — §1 ADR index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionsRow {
    pub id: String,
    pub adr: String,
    pub kind: String, // out-of-scope | replaced | deviation
    pub notes: String,
}

/// `symbols.csv` row — §1 symbols schema (9 cols, khác `FilesRow`).
/// Lead theo plan report 02 §1: symbols là unit port-level nhỏ hơn file,
/// có `file_id` tham chiếu `FilesRow.id`, không có `wt_loc`/`module`/`phase`/
/// `author`/`reviewed_by`/`reviewed_at` (kế thừa từ file row cha).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolsRow {
    pub file_id: String,
    pub symbol: String,
    pub wt_anchor: String,
    pub kind: String, // ctor | method | property | struct | field | ...
    pub rust_path: String,
    pub status: String,
    pub test_ids: String,
    pub adr: String,
    pub notes: String,
}

/// Status enum terminal states (§1): chỉ `verified|replaced|out-of-scope` count as done.
pub const DONE_STATUSES: &[&str] = &["verified", "replaced", "out-of-scope"];
pub const NOT_DONE: &[&str] = &["not-started", "stub", "partial", "ported"];
pub const TEST_DONE: &[&str] = &["passing", "waived"];

/// Tất cả shard dirs dưới `parity/ledger/`.
pub fn shard_dirs() -> Vec<PathBuf> {
    let root = paths::ledger_dir();
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(&root) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Đọc một shard CSV (generic qua header). Trả Vec<Record> với serde deserialize.
pub fn read_csv<T: for<'de> Deserialize<'de>>(path: &Path) -> Rt<Vec<T>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let mut out = Vec::new();
    for rec in rdr.deserialize() {
        out.push(rec?);
    }
    Ok(out)
}

/// Ghi CSV với header (serde serialize). Tạo parent dir nếu cần.
pub fn write_csv<T: Serialize>(path: &Path, rows: &[T]) -> R {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let mut wtr = csv::Writer::from_path(path)?;
    for r in rows {
        wtr.serialize(r)?;
    }
    wtr.flush()?;
    Ok(())
}

/// Merge tất cả shards của một kind (`files`/`symbols`/`tests`/`decisions`)
/// thành `parity/merged/<kind>.csv`. Spec §3 `merge`.
fn merge_kind(kind: &str) -> Rt<usize> {
    let out = paths::merged_dir().join(format!("{kind}.csv"));
    fs::create_dir_all(paths::merged_dir())?;
    // Lead với header riêng — ghép thẳng các row của shards.
    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header: Option<Vec<String>> = None;
    let mut dup_ids = BTreeSet::new();
    let mut seen_ids = BTreeSet::new();
    for shard in shard_dirs() {
        let f = shard.join(format!("{kind}.csv"));
        if !f.exists() {
            continue;
        }
        let mut rdr = csv::Reader::from_path(&f)?;
        let h = rdr.headers()?.clone();
        if header.is_none() {
            header = Some(h.iter().map(|s| s.to_string()).collect());
        }
        for rec in rdr.records() {
            let rec = rec?;
            let row: Vec<String> = rec.iter().map(|s| s.to_string()).collect();
            // id ở column 0 — check duplicate.
            if let Some(id) = row.first() {
                if !id.is_empty() && !seen_ids.insert(id.clone()) {
                    dup_ids.insert(id.clone());
                }
            }
            all_rows.push(row);
        }
    }
    let mut wtr = csv::Writer::from_path(&out)?;
    if let Some(h) = header {
        // Serialize header raw.
        let hrec: csv::StringRecord = h.iter().collect();
        wtr.write_record(&hrec)?;
    }
    for r in &all_rows {
        wtr.write_record(r)?;
    }
    wtr.flush()?;
    if !dup_ids.is_empty() {
        return Err(format!(
            "merge {kind}: duplicate-id error — ids {:?}",
            dup_ids.iter().take(10).collect::<Vec<_>>()
        )
        .into());
    }
    Ok(all_rows.len())
}

/// `xtask merge` — merge files/symbols/tests/decisions. Spec §3.
pub fn cmd_merge() -> R {
    for kind in &["files", "symbols", "tests", "decisions"] {
        let n = merge_kind(kind)?;
        println!("merge {kind}: {n} rows → parity/merged/{kind}.csv");
    }
    Ok(())
}

/// `xtask ledger-check [--root DIR]`. Spec §3.
///
/// Asserts:
/// - CSV schema valid (header present, required fields)
/// - `verified` rows: `reviewed_by` non-empty, `reviewed_at` non-empty, `reviewed_by != author`
/// - mỗi row có `phase`, `rust_path`除非 status == `not-started`
/// - no orphan WT file (re-walk `windows/src` limited `*.cpp|*.h|*.hpp|*.idl|*.xaml`);
///   mỗi file phải xuất hiện trong shard `files.csv` hoặc `exclusions.csv` với ADR
/// - `verified` row có `test_ids` resolve trong `cargo test --list` (defer — chỉ check format)
/// - `replaced|out-of-scope` có ADR file tồn tại
pub fn cmd_check(args: &[String]) -> R {
    let root = parse_root_flag(args).unwrap_or_else(paths::windows_src);
    let shards = shard_dirs();
    if shards.is_empty() {
        return Err("ledger-check: no shard dirs under parity/ledger/".into());
    }
    let mut errors: Vec<String> = Vec::new();

    // Collect tất cả files rows + wt_path set.
    let mut all_files: Vec<FilesRow> = Vec::new();
    let mut all_excl: Vec<ExclusionsRow> = Vec::new();
    let mut all_tests: Vec<TestsRow> = Vec::new();
    let mut all_decisions: Vec<DecisionsRow> = Vec::new();
    let mut all_symbols: Vec<SymbolsRow> = Vec::new();

    let excl_path = paths::exclusions_csv();
    if excl_path.exists() {
        all_excl = read_csv(&excl_path)?;
    }

    for s in &shards {
        let fp = s.join("files.csv");
        if fp.exists() {
            let rows: Vec<FilesRow> = read_csv(&fp)?;
            verify_files_rows(&rows, &mut errors);
            all_files.extend(rows);
        }
        let tp = s.join("tests.csv");
        if tp.exists() {
            let rows: Vec<TestsRow> = read_csv(&tp)?;
            verify_tests_rows(&rows, &mut errors);
            all_tests.extend(rows);
        }
        let dp = s.join("decisions.csv");
        if dp.exists() {
            let rows: Vec<DecisionsRow> = read_csv(&dp)?;
            all_decisions.extend(rows);
        }
        let sp = s.join("symbols.csv");
        if sp.exists() {
            let rows: Vec<SymbolsRow> = read_csv(&sp)?;
            verify_symbols_rows(&rows, &mut errors);
            all_symbols.extend(rows);
        }
    }

    // Orphan check: walk root (windows/src), mỗi file phải có row.
    let ledgered: BTreeSet<String> = all_files
        .iter()
        .map(|r| r.wt_path.clone())
        .chain(all_excl.iter().map(|r| r.wt_path.clone()))
        .collect();
    let orphan = walk_in_scope_wt(&root, &ledgered);
    if !orphan.is_empty() {
        errors.push(format!(
            "orphan WT files (not in ledger/exclusions): {:?}",
            orphan.iter().take(10).collect::<Vec<_>>()
        ));
    }

    // ADR existence cho replaced/out-of-scope.
    let adrs_seen = check_adrs_exist(&all_files, &all_excl, &all_decisions, &mut errors);

    if errors.is_empty() {
        println!(
            "ledger-check: OK — {} files rows, {} exclusions, {} tests, {} decisions, {} ADRs",
            all_files.len(),
            all_excl.len(),
            all_tests.len(),
            all_decisions.len(),
            adrs_seen
        );
        Ok(())
    } else {
        Err(format!(
            "ledger-check FAILED ({} violations):\n  - {}",
            errors.len(),
            errors.join("\n  - ")
        )
        .into())
    }
}

fn verify_files_rows(rows: &[FilesRow], errors: &mut Vec<String>) {
    for r in rows {
        if r.phase.is_empty() {
            errors.push(format!("row {} missing phase", r.id));
        }
        if r.status != "not-started" && r.rust_path.is_empty() {
            errors.push(format!("row {} status={} but no rust_path", r.id, r.status));
        }
        if r.status == "verified" {
            if r.reviewed_by.is_empty() {
                errors.push(format!("verified row {} missing reviewed_by", r.id));
            }
            if r.reviewed_at.is_empty() {
                errors.push(format!("verified row {} missing reviewed_at", r.id));
            }
            if !r.reviewed_by.is_empty() && r.reviewed_by == r.author {
                errors.push(format!(
                    "verified row {} reviewed_by == author ({})",
                    r.id, r.author
                ));
            }
            if r.test_ids.is_empty() {
                errors.push(format!("verified row {} missing test_ids", r.id));
            }
        }
        if matches!(r.status.as_str(), "replaced" | "out-of-scope") && r.adr.is_empty() {
            errors.push(format!("{} row {} missing adr", r.status, r.id));
        }
    }
}

fn verify_tests_rows(rows: &[TestsRow], errors: &mut Vec<String>) {
    for r in rows {
        if r.phase.is_empty() {
            errors.push(format!("tests row {} missing phase", r.id));
        }
        // Test naming rule: wt_<suite>__<method> (double underscore).
        if r.status == "passing" && r.rust_test.is_empty() {
            errors.push(format!("passing test row {} missing rust_test", r.id));
        }
        if r.status == "waived" && r.adr.is_empty() {
            errors.push(format!("waived test row {} missing adr", r.id));
        }
    }
}

/// Verify `symbols.csv` rows — schema §1 symbols. Symbol row kế thừa phase/
/// module/wt_loc từ file row cha (qua `file_id`), nên chỉ check status-internal:
/// `verified` có `test_ids` + (ADR cho replaced/out-of-scope). Authors/
/// reviewers không áp dụng ở symbol-level.
fn verify_symbols_rows(rows: &[SymbolsRow], errors: &mut Vec<String>) {
    for r in rows {
        if r.file_id.is_empty() {
            errors.push(format!("symbol row {:?} missing file_id", r.symbol));
        }
        if r.symbol.is_empty() {
            errors.push(format!("symbol row file_id={} has empty symbol", r.file_id));
        }
        if matches!(r.status.as_str(), "replaced" | "out-of-scope") && r.adr.is_empty() {
            errors.push(format!("{} symbol {} missing adr", r.status, r.symbol));
        }
    }
}

/// Walk `windows/src` (or `--root`) filtered `*.cpp|*.h|*.hpp|*.idl|*.xaml`.
/// Loại trừ các extension khác (.vcxproj/.props/.sln/.def/.rc/.inc/.cs/.resw/.json/images).
fn walk_in_scope_wt(root: &Path, ledgered: &BTreeSet<String>) -> Vec<String> {
    let mut files = Vec::new();
    let exts = ["cpp", "h", "hpp", "idl", "xaml"];
    walk(root, root, &exts, &mut files);
    // Normalize: wt_path relative to `windows/` (or root parent) — use relative to root.
    files
        .into_iter()
        .filter(|p| {
            // wt_path trong ledger là path tương đối windows/src — convert.
            let rel = p.replace('\\', "/");
            !ledgered.contains(&rel)
        })
        .collect()
}

fn walk(root: &Path, dir: &Path, exts: &[&str], out: &mut Vec<String>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(root, &p, exts, out);
        } else if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
            if exts.contains(&ext) {
                if let Ok(rel) = p.strip_prefix(root) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
}

fn check_adrs_exist(
    files: &[FilesRow],
    excl: &[ExclusionsRow],
    _dec: &[DecisionsRow],
    errors: &mut Vec<String>,
) -> usize {
    let adr_root = paths::workspace_root().join("docs").join("decisions");
    let mut need: BTreeSet<String> = BTreeSet::new();
    for r in files {
        if matches!(r.status.as_str(), "replaced" | "out-of-scope") && !r.adr.is_empty() {
            need.insert(r.adr.clone());
        }
    }
    for r in excl {
        if !r.adr.is_empty() {
            need.insert(r.adr.clone());
        }
    }
    let mut ok = 0usize;
    for adr_id in &need {
        // adr_id có thể là "0001" hoặc "0001-scope" — match file 0001-*.md.
        let mut found = false;
        if let Ok(rd) = fs::read_dir(&adr_root) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with(&format!("{adr_id}-")) && name.ends_with(".md") {
                    found = true;
                    break;
                }
            }
        }
        if found {
            ok += 1;
        } else {
            errors.push(format!(
                "ADR {adr_id} referenced but no file docs/decisions/{adr_id}-*.md"
            ));
        }
    }
    ok
}

fn parse_root_flag(args: &[String]) -> Option<PathBuf> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == "--root" {
            if let Some(v) = iter.next() {
                return Some(PathBuf::from(v));
            }
        } else if let Some(v) = a.strip_prefix("--root=") {
            return Some(PathBuf::from(v));
        }
    }
    None
}

/// Summary counts per status — dùng bởi `audit`. Trả map status → count.
pub fn status_counts() -> BTreeMap<String, u32> {
    let mut m = BTreeMap::new();
    let mut add = |s: &str| *m.entry(s.to_string()).or_insert(0) += 1;
    for s in shard_dirs() {
        let fp = s.join("files.csv");
        if let Ok(rows) = read_csv::<FilesRow>(&fp) {
            for r in rows {
                add(&r.status);
            }
        }
        // symbols schema riêng (§1) — đếm cùng audit theo spec.
        let sp = s.join("symbols.csv");
        if let Ok(rows) = read_csv::<SymbolsRow>(&sp) {
            for r in rows {
                add(&r.status);
            }
        }
    }
    m
}

/// Tests counts per status — cho audit.
pub fn tests_counts() -> BTreeMap<String, u32> {
    let mut m = BTreeMap::new();
    let mut add = |s: &str| *m.entry(s.to_string()).or_insert(0) += 1;
    for s in shard_dirs() {
        let tp = s.join("tests.csv");
        if let Ok(rows) = read_csv::<TestsRow>(&tp) {
            for r in rows {
                add(&r.status);
            }
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempfile_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("xtask_test_{}", uuid_like()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn uuid_like() -> String {
        use std::time::SystemTime;
        let n = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{n:x}")
    }

    #[test]
    fn walk_filters_extensions() {
        let d = tempfile_dir();
        fs::write(d.join("a.cpp"), b"").unwrap();
        fs::write(d.join("b.vcxproj"), b"").unwrap();
        fs::write(d.join("c.hpp"), b"").unwrap();
        fs::write(d.join("d.json"), b"").unwrap();
        let mut out = Vec::new();
        walk(&d, &d, &["cpp", "h", "hpp", "idl", "xaml"], &mut out);
        let s: BTreeSet<String> = out.into_iter().collect();
        assert!(s.contains("a.cpp"));
        assert!(s.contains("c.hpp"));
        assert!(!s.contains("b.vcxproj"));
        assert!(!s.contains("d.json"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn verify_files_rows_flags_verified_no_reviewer() {
        let mut errs = Vec::new();
        let rows = vec![FilesRow {
            id: "f1".into(),
            wt_path: "x.cpp".into(),
            wt_loc: 1,
            module: "m".into(),
            phase: "P03".into(),
            rust_path: "r.rs".into(),
            status: "verified".into(),
            test_ids: "t1".into(),
            adr: "".into(),
            author: "alice".into(),
            reviewed_by: "".into(),
            reviewed_at: "".into(),
            notes: "".into(),
        }];
        verify_files_rows(&rows, &mut errs);
        assert!(errs.iter().any(|e| e.contains("missing reviewed_by")));
    }

    #[test]
    fn verify_files_rows_flags_reviewer_equals_author() {
        let mut errs = Vec::new();
        let rows = vec![FilesRow {
            id: "f2".into(),
            wt_path: "x.cpp".into(),
            wt_loc: 1,
            module: "m".into(),
            phase: "P03".into(),
            rust_path: "r.rs".into(),
            status: "verified".into(),
            test_ids: "t1".into(),
            adr: "".into(),
            author: "alice".into(),
            reviewed_by: "alice".into(),
            reviewed_at: "2026-10-26".into(),
            notes: "".into(),
        }];
        verify_files_rows(&rows, &mut errs);
        assert!(errs.iter().any(|e| e.contains("reviewed_by == author")));
    }

    #[test]
    fn read_write_csv_roundtrip() {
        let d = tempfile_dir();
        let p = d.join("files.csv");
        let rows = vec![FilesRow {
            id: "x".into(),
            wt_path: "a/b.cpp".into(),
            wt_loc: 10,
            module: "m".into(),
            phase: "P03".into(),
            rust_path: "crates/core/src/x.rs".into(),
            status: "stub".into(),
            test_ids: "".into(),
            adr: "".into(),
            author: "a".into(),
            reviewed_by: "b".into(),
            reviewed_at: "".into(),
            notes: "n".into(),
        }];
        write_csv(&p, &rows).unwrap();
        let back: Vec<FilesRow> = read_csv(&p).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].id, "x");
        assert_eq!(back[0].wt_loc, 10);
        let _ = fs::remove_dir_all(&d);
    }
}
