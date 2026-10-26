//! Manual arg parse — std only, no clap (frozen deps).
//! `cargo xtask <command> [flags]`.

use crate::R;
use crate::{audit, cov, e2e, include_graph, ledger, loc_ratio, seed, stubs, tests_check};

/// Dispatch theo subcommand. Spec report 02 §3 bảng commands.
pub fn dispatch(args: Vec<String>) -> R {
    // args[0] = "xtask" (bin name hoặc alias). Lấy sub tại [1].
    if args.len() < 2 {
        return usage_err();
    }
    let cmd = args[1].as_str();
    let rest = &args[2..];
    match cmd {
        "audit" => audit::cmd_audit(rest),
        "ledger-seed" => seed::cmd_seed(rest),
        "merge" => ledger::cmd_merge(),
        "cov" => cov::cmd_cov(rest),
        "e2e" => e2e::cmd_e2e(rest),
        "stubs" => stubs::cmd_stubs(rest),
        "ledger-check" => ledger::cmd_check(rest),
        "tests-check" => tests_check::cmd_tests_check(rest),
        "include-graph" => include_graph::cmd_include_graph(rest),
        "loc-ratio" => loc_ratio::cmd_loc_ratio(rest),
        // P19 sẽ append `summary`/`fuzz` glue — chưa có ở P01.
        _ => usage_err(),
    }
}

fn usage_err() -> R {
    Err("usage: xtask <command> [args]\n\
         commands: audit --wave N | --final, ledger-seed [--tests], merge,\n\
         cov --phase NN, e2e, stubs [--path DIR], ledger-check [--root DIR],\n\
         tests-check, include-graph, loc-ratio"
        .to_string()
        .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_unknown_returns_err() {
        let args = vec!["xtask".into(), "nope".into()];
        assert!(dispatch(args).is_err());
    }

    #[test]
    fn dispatch_empty_returns_err() {
        let args = vec!["xtask".into()];
        assert!(dispatch(args).is_err());
    }
}
