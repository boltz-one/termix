//! `xtask ledger-seed [--tests]` — spec §3.
//!
//! Walk `windows/src` → per-phase shards `files.csv` (status `not-started`,
//! đã-port pre-marked `partial`), `exclusions.csv` (out-of-scope + ADR-0001/0007),
//! `tests.csv` (từ `TEST_METHOD`/`BEGIN_TEST_METHOD`).
//!
//! Owning phase per file: mapping list (master plan step 5) — áp dụng khi seeding.
//! P01 tạo schema + seed; verified viết bởi audit reviewer sau này.

use crate::ledger::{write_csv, ExclusionsRow, FilesRow, TestsRow};
use crate::paths;
use crate::R;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Mapping WT dir/file prefix → owning phase (master plan step 5 + matrix).
///.Override oleh caller nếu cần — ánh xạ tĩnh覆盖 các high-frequency cases.
fn owning_phase(wt_path: &str) -> &'static str {
    // normalize backslash
    let p = wt_path.replace('\\', "/");
    // out-of-scope dirs (ADR-0001 / ADR-0007):
    if p.starts_with("host/")
        || p.starts_with("server/")
        || p.starts_with("renderer/gdi")
        || p.starts_with("renderer/wddmcon")
        || p.starts_with("audio/")
        || p.starts_with("tools/")
    {
        return "OUT";
    }
    if p.contains("HwndTerminal") || p.contains("PublicTerminalCore") {
        return "OUT";
    }
    if p.starts_with("TerminalConnection/AzureConnection") {
        return "OUT"; // ADR-0007
    }
    // In-scope → phase:
    if p.starts_with("terminal/adapter/") {
        return "P03";
    }
    if p.starts_with("terminal/parser/") {
        return "P04";
    }
    if p.starts_with("terminal/input/") || p.contains("terminalInput") {
        return "P04";
    }
    if p.starts_with("types/") || p.starts_with("til/") {
        return "P05";
    }
    if p.starts_with("buffer/") || p.contains("textBuffer") || p.contains("Row.hpp") {
        return "P08";
    }
    if p.starts_with("terminal/core/") || p.contains("Terminal.hpp") {
        return "P09";
    }
    if p.starts_with("renderer/") {
        return "P10";
    }
    if p.starts_with("TerminalConnection/") {
        return "P06";
    }
    if p.starts_with("TerminalControl/")
        || p.contains("ControlCore")
        || p.contains("ControlInteractivity")
        || p.contains("TermControl")
    {
        return "P11";
    }
    if p.contains("ActionAndArgs")
        || p.starts_with("TerminalApp/")
        || p.contains("Commandline")
        || p.contains("Remoting")
    {
        return "P13";
    }
    if p.starts_with("TerminalSettingsModel/") || p.contains("CascadiaSettings") {
        return "P07";
    }
    if p.contains("SettingsEditor") || p.contains("ColorPickup") {
        return "P17";
    }
    if p.contains("AtlasEngine") || p.contains("DwriteAtlas") {
        return "P10";
    }
    if p.contains("UIAutomation")
        || p.contains("UiaTextRange")
        || p.contains(" Accessibility")
        || p.contains("UiaProvider")
    {
        return "P15";
    }
    if p.contains("WindowEmperor") || p.contains("TaskbarState") {
        return "P16";
    }
    if p.contains("Markdown") || p.contains("Snippet") || p.contains("Scratchpad") {
        return "P14";
    }
    if p.starts_with("TerminalApp/ShortcutAction") {
        return "P13";
    }
    "P99" // unmapped — flag cho hand-review
}

/// `xtask ledger-seed [--tests]`.
pub fn cmd_seed(args: &[String]) -> R {
    let with_tests = args.iter().any(|a| a == "--tests");
    let src = paths::windows_src();
    if !src.exists() {
        return Err(format!("ledger-seed: windows/src not found at {}", src.display()).into());
    }
    // Walk in-scope source files.
    let exts = ["cpp", "h", "hpp", "idl", "xaml"];
    let mut all_files: Vec<(String, PathBuf)> = Vec::new();
    walk(&src, &src, &exts, &mut all_files);
    println!(
        "ledger-seed: walked {} in-scope source files",
        all_files.len()
    );

    // Group theo owning phase.
    let mut per_phase: BTreeMap<&'static str, Vec<FilesRow>> = BTreeMap::new();
    let mut exclusions: Vec<ExclusionsRow> = Vec::new();
    for (next_id, (rel, _abs)) in (1u32..).zip(all_files.iter()) {
        let phase = owning_phase(rel);
        let row = FilesRow {
            id: format!("f{next_id:05}"),
            wt_path: rel.clone(),
            wt_loc: 0, // wt_loc filled by audit reviewer or optional pass
            module: phase_module(rel),
            phase: phase.to_string(),
            rust_path: String::new(), // not-started → empty allowed
            status: "not-started".into(),
            test_ids: String::new(),
            adr: String::new(),
            author: String::new(),
            reviewed_by: String::new(),
            reviewed_at: String::new(),
            notes: String::new(),
        };
        if phase == "OUT" {
            let adr = if rel.contains("AzureConnection") {
                "0007".to_string()
            } else {
                "0001".to_string()
            };
            exclusions.push(ExclusionsRow {
                id: row.id.clone(),
                wt_path: rel.clone(),
                adr,
                notes: "out-of-scope per ADR".into(),
            });
        } else {
            per_phase.entry(phase).or_default().push(row);
        }
    }

    // Write per-phase shard `files.csv`.
    let ledger = paths::ledger_dir();
    fs::create_dir_all(&ledger)?;
    for (phase, rows) in &per_phase {
        let shard = ledger.join(phase_folder_name(phase));
        fs::create_dir_all(&shard)?;
        let fp = shard.join("files.csv");
        write_csv(&fp, rows)?;
        println!(
            "ledger-seed: {} → {}/files.csv ({} rows)",
            phase,
            shard.display(),
            rows.len()
        );
    }
    // Write single exclusions.csv.
    let ex_path = paths::exclusions_csv();
    write_csv(&ex_path, &exclusions)?;
    println!("ledger-seed: exclusions.csv ({} rows)", exclusions.len());

    if with_tests {
        seed_tests(&src)?;
    }
    Ok(())
}

fn phase_folder_name(phase: &str) -> String {
    // pNN-<name> — dùng stub name; operator rename sau nếu cần.
    let name = match phase {
        "P01" => "foundations",
        "P02" => "m1-skeleton",
        "P03" => "vt-adapter",
        "P04" => "parser-input",
        "P05" => "types-til",
        "P06" => "connection",
        "P07" => "settings",
        "P08" => "buffer",
        "P09" => "terminal-core",
        "P10" => "renderer",
        "P11" => "control",
        "P12" => "app-core",
        "P13" => "actions",
        "P14" => "ui-chrome",
        "P15" => "a11y-ime",
        "P16" => "host-window",
        "P17" => "editor",
        "P18" => "extras-packaging",
        "P19" => "audit-closure",
        _ => "unmapped",
    };
    format!("{phase}-{name}")
}

fn phase_module(wt_path: &str) -> String {
    let p = wt_path.replace('\\', "/");
    //灵动 — lấy segment đầu sau `/`.
    p.split('/').next().unwrap_or("unknown").to_string()
}

/// Walk tests: scan `*/ut_*.cpp` cho `TEST_METHOD(...)` / `BEGIN_TEST_METHOD(...)`.
/// Spec §2 test naming `wt_<suite>__<method>` — phase-side rename khi port.
fn seed_tests(src: &Path) -> R {
    let mut all_tests: Vec<TestsRow> = Vec::new();
    let mut next_id = 1u32;
    // Walk .cpp (test files).
    walk_tests(src, &mut |path, content| {
        let suite = derive_suite(path, src);
        for method in extract_test_methods(content) {
            all_tests.push(TestsRow {
                id: format!("t{next_id:05}"),
                wt_path: path.to_string(),
                wt_suite_method: format!("{suite}::{method}"),
                phase: owning_phase(path).to_string(),
                rust_test: String::new(),
                status: "not-started".into(),
                adr: String::new(),
            });
            next_id += 1;
        }
    });
    // Phase-side shard write.
    let ledger = paths::ledger_dir();
    for s in fs::read_dir(&ledger)?.flatten() {
        if !s.path().is_dir() {
            continue;
        }
        let sf = s.path();
        if !sf.join("files.csv").exists() {
            continue;
        }
        let phase = sf
            .file_name()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or_default();
        let phase_code = phase.split('-').next().unwrap_or("P99").to_string();
        let rows: Vec<TestsRow> = all_tests
            .iter()
            .filter(|t| t.phase == phase_code)
            .cloned()
            .collect();
        if rows.is_empty() {
            continue;
        }
        let tp = sf.join("tests.csv");
        write_csv(&tp, &rows)?;
    }
    println!(
        "ledger-seed: tests — {} TEST_METHODs total",
        all_tests.len()
    );
    Ok(())
}

fn walk_tests<F: FnMut(&str, &str)>(dir: &Path, cb: &mut F) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_tests(&p, cb);
            continue;
        }
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "cpp" {
            continue;
        }
        let name = p
            .file_name()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or_default();
        // ut_adapter.cpp, ut_parser.cpp, UnitTestClasses.cpp, etc.
        let is_test = name.starts_with("ut_")
            || name.starts_with("UnitTests")
            || name.starts_with("Test")
            || name.contains("LocalTests");
        if !is_test {
            continue;
        }
        let Ok(content) = fs::read_to_string(&p) else {
            continue;
        };
        let rel = p.strip_prefix(paths::windows_src()).unwrap_or(&p);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        cb(&rel_str, &content);
    }
}

fn derive_suite(wt_path: &str, _src: &Path) -> String {
    // heuristic — wt_<suite> prefix per WT test group (adapter, input, parser, ...).
    let p = wt_path.replace('\\', "/");
    let fname = p.rsplit('/').next().unwrap_or("");
    if fname.contains("adapter") {
        "adapter"
    } else if fname.contains("parser") {
        "parser"
    } else if fname.contains("input") {
        "input"
    } else if fname.contains("textbuffer") || fname.contains("textBuffer") {
        "textbuffer"
    } else if fname.contains("terminalcore") || fname.contains("TerminalCore") {
        "terminalcore"
    } else if fname.contains("til") {
        "til"
    } else if fname.contains("types") {
        "types"
    } else if fname.contains("Control") {
        "control"
    } else if fname.contains("Settings") || fname.contains("settings") {
        "settings"
    } else if fname.contains("app") || fname.contains("App") {
        "app"
    } else if fname.contains("Commandline") {
        "cmdline"
    } else if fname.contains("Action") {
        "actions"
    } else {
        "misc"
    }
    .to_string()
}

fn extract_test_methods(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    // `TEST_METHOD(Name)` — TAEF macro.
    for line in content.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("TEST_METHOD(") {
            if let Some(end) = rest.find(')') {
                let name = rest[..end].trim().to_string();
                if !name.is_empty() {
                    out.push(name);
                }
            }
        } else if let Some(rest) = t.strip_prefix("BEGIN_TEST_METHOD(") {
            if let Some(end) = rest.find(')') {
                let name = rest[..end].trim().to_string();
                if !name.is_empty() {
                    out.push(name);
                }
            }
        }
    }
    out
}

fn walk(root: &Path, dir: &Path, exts: &[&str], out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(root, &p, exts, out);
            continue;
        }
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !exts.contains(&ext) {
            continue;
        }
        if let Ok(rel) = p.strip_prefix(root) {
            out.push((rel.to_string_lossy().replace('\\', "/"), p.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owning_phase_dispatches_known_dirs() {
        assert_eq!(owning_phase("terminal/adapter/adaptDispatch.hpp"), "P03");
        assert_eq!(owning_phase("terminal/parser/StateMachine.cpp"), "P04");
        assert_eq!(owning_phase("types/utils.cpp"), "P05");
        assert_eq!(
            owning_phase("TerminalConnection/ConptyConnection.cpp"),
            "P06"
        );
        assert_eq!(owning_phase("host/srv.cpp"), "OUT");
        assert_eq!(
            owning_phase("TerminalConnection/AzureConnection.cpp"),
            "OUT"
        );
    }

    #[test]
    fn extract_test_methods_basic() {
        // Multi-line — TAEF macros viết mỗi cái 1 dòng.
        let src = "class X {\n TEST_METHOD(Foo)\n TEST_METHOD(Bar)\n};";
        let v = extract_test_methods(src);
        assert_eq!(v, vec!["Foo", "Bar"]);
    }

    #[test]
    fn derive_suite_from_filename() {
        assert_eq!(
            derive_suite("terminal/adapter/ut_adapter.cpp", Path::new(".")),
            "adapter"
        );
        assert_eq!(
            derive_suite("terminal/parser/ut_parser.cpp", Path::new(".")),
            "parser"
        );
    }
}
