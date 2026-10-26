//! `xtask` — parity audit tooling cho WT→termix port (P01 tạo).
//!
//! Port: spec `reports/02-parity-ledger-and-audit-loop.md` §3 — full command set.
//! Deps: std + csv + serde + serde_json only (frozen). P19 append-only summary/fuzz.
//!
//! Lệnh: `audit --wave/--final`, `ledger-seed [--tests]`, `merge`, `cov --phase`,
//! `e2e`, `stubs [--path]`, `ledger-check [--root]`, `tests-check`, `include-graph`,
//! `loc-ratio`. Alias `.cargo/config.toml`: `xtask = "run -p xtask --"`.
//!
//! Chạy từ git root `terminal/` (parent của `terminal_gpui/`). `windows/` read-only.

pub mod audit;
pub mod cli;
pub mod cov;
pub mod e2e;
pub mod include_graph;
pub mod ledger;
pub mod loc_ratio;
pub mod paths;
pub mod seed;
pub mod stubs;
pub mod summary;
pub mod tests_check;

/// Đường dẫn để exit code khác 0 — error được propagate lên main.
/// `R` = `Result<(), Error>` cho CSV/write ops; `Rt<T>` cho ops trả data.
pub type R = Result<(), Box<dyn std::error::Error>>;
pub type Rt<T> = Result<T, Box<dyn std::error::Error>>;

/// Chạy từ entry bin. Trả exit code (0 = ok).
pub fn run(args: Vec<String>) -> i32 {
    match cli::dispatch(args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("xtask error: {e}");
            1
        }
    }
}
