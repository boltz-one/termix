//! `xtask e2e` — spec §3.
//!
//! Runs rows of `parity/e2e.csv` (scenario, phase, command, ci|manual).
//! Executes mỗi `ci` row; `manual` row cần checklist file tồn tại signed.

use crate::paths;
use crate::R;
use std::fs;
use std::process::Command;

pub fn cmd_e2e(_args: &[String]) -> R {
    let csv = paths::e2e_csv();
    if !csv.exists() {
        return Err("e2e: parity/e2e.csv not found".into());
    }
    let mut rdr = csv::Reader::from_path(&csv)?;
    let mut fails = Vec::new();
    let mut total = 0u32;
    for rec in rdr.records() {
        let rec = rec?;
        // columns: scenario, phase, command, kind
        let scenario = rec.get(0).unwrap_or("").to_string();
        let phase = rec.get(1).unwrap_or("").to_string();
        let command = rec.get(2).unwrap_or("").to_string();
        let kind = rec.get(3).unwrap_or("").to_string();
        if scenario.is_empty() {
            continue;
        }
        total += 1;
        match kind.as_str() {
            "ci" => {
                let parts = shell_split(&command);
                if parts.is_empty() {
                    fails.push(format!("{scenario}: empty command"));
                    continue;
                }
                let out = Command::new(&parts[0]).args(&parts[1..]).output();
                match out {
                    Ok(o) if o.status.success() => {
                        println!("e2e {scenario} ({phase}): OK");
                    }
                    Ok(o) => fails.push(format!(
                        "{scenario} ({phase}): exit {:?}\n{}",
                        o.status.code(),
                        String::from_utf8_lossy(&o.stderr)
                    )),
                    Err(e) => fails.push(format!("{scenario} ({phase}): spawn: {e}")),
                }
            }
            "manual" => {
                let checklist = paths::reports_dir()
                    .join(format!("p{phase}-{}-checklist.md", slugify(&scenario)));
                if !checklist.exists() {
                    fails.push(format!(
                        "{scenario} ({phase}): manual checklist missing at {}",
                        checklist.display()
                    ));
                    continue;
                }
                let s = fs::read_to_string(&checklist)?;
                if !s.contains("[x]") && !s.contains("[X]") {
                    fails.push(format!(
                        "{scenario} ({phase}): checklist not signed ([ ] only)"
                    ));
                } else {
                    println!("e2e {scenario} ({phase}): manual checklist signed");
                }
            }
            _ => fails.push(format!("{scenario} ({phase}): unknown kind `{kind}`")),
        }
    }
    if fails.is_empty() {
        println!("e2e: {total} scenarios OK");
        Ok(())
    } else {
        Err(format!(
            "e2e FAILED ({} of {total}):\n  - {}",
            fails.len(),
            fails.join("\n  - ")
        )
        .into())
    }
}

fn shell_split(s: &str) -> Vec<String> {
    // Simple whitespace split — assumes no quoted args (xtask commands are simple).
    s.split_whitespace().map(String::from).collect()
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
}
