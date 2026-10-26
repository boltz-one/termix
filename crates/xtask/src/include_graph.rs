//! `xtask include-graph` — spec §3.
//!
//! Grep IN-scope C++ for `#include` of OUT-of-scope dirs → candidate promotions.
//! Fail nếu referenced OUT file thiếu IN promotion row hoặc ADR.

use crate::ledger::{read_csv, ExclusionsRow, FilesRow};
use crate::paths;
use crate::R;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub fn cmd_include_graph(_args: &[String]) -> R {
    let src = paths::windows_src();
    // OUT-of-scope dir prefixes (ADR-0001).
    let out_dirs = [
        "host/",
        "server/",
        "renderer/gdi",
        "renderer/wddmcon",
        "audio/",
        "tools/",
    ];
    let mut references: BTreeSet<String> = BTreeSet::new();
    walk_cpp(&src, &src, &mut |path, content| {
        for line in content.lines() {
            let t = line.trim();
            if let Some(rest) = t
                .strip_prefix("#include \"")
                .and_then(|r| r.strip_suffix('"'))
                .or_else(|| {
                    t.strip_prefix("#include <")
                        .and_then(|r| r.strip_suffix('>'))
                })
            {
                let norm = rest.replace('\\', "/");
                if out_dirs.iter().any(|d| norm.starts_with(d))
                    || norm.contains("HwndTerminal")
                    || norm.contains("PublicTerminalCore")
                {
                    references.insert(norm);
                }
            }
        }
        let _ = path;
    });
    // Each referenced OUT file needs IN promotion row in a shard `files.csv`
    // (status replaced, adr) hoặc ADR row.
    let mut promoted: BTreeSet<String> = BTreeSet::new();
    for s in crate::ledger::shard_dirs() {
        let fp = s.join("files.csv");
        if fp.exists() {
            if let Ok(rows) = read_csv::<FilesRow>(&fp) {
                for r in rows {
                    if r.status == "replaced" || r.status == "out-of-scope" {
                        promoted.insert(r.wt_path);
                    }
                }
            }
        }
    }
    let excl_path = paths::exclusions_csv();
    let excluded: BTreeSet<String> = if excl_path.exists() {
        read_csv::<ExclusionsRow>(&excl_path)
            .unwrap_or_default()
            .into_iter()
            .map(|r| r.wt_path)
            .collect()
    } else {
        BTreeSet::new()
    };
    let unledgered: Vec<String> = references
        .iter()
        .filter(|p| !promoted.contains(*p) && !excluded.contains(*p))
        .cloned()
        .collect();
    if unledgered.is_empty() {
        println!(
            "include-graph: OK ({} OUT-scope references all promoted/excluded)",
            references.len()
        );
        Ok(())
    } else {
        Err(format!(
            "include-graph FAILED — {} referenced OUT files lacking promotion/ADR:\n  - {}",
            unledgered.len(),
            unledgered.join("\n  - ")
        )
        .into())
    }
}

fn walk_cpp<F: FnMut(&str, &str)>(root: &Path, dir: &Path, cb: &mut F) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_cpp(root, &p, cb);
        } else if matches!(
            p.extension().and_then(|s| s.to_str()),
            Some("cpp") | Some("h") | Some("hpp")
        ) {
            if let Ok(s) = fs::read_to_string(&p) {
                let rel = p.strip_prefix(root).unwrap_or(&p);
                cb(&rel.to_string_lossy().replace('\\', "/"), &s);
            }
        }
    }
}
