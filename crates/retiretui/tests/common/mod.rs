//! Helpers the binary's test crates share.
#![allow(dead_code, reason = "each test crate uses its own share of these")]

pub mod mcp;

use std::path::PathBuf;
use std::process::{Command, Output};

pub const FULL_PLAN: &str = "../retiretui_engine/tests/fixtures/full.toml";
pub const OPT_PLAN: &str = include_str!("../fixtures/opt-plan.toml");
pub const CLAIMS_PLAN: &str = include_str!("../fixtures/claims-plan.toml");
pub const STATEMENT: &str = "../retiretui_engine/tests/fixtures/statement.xml";

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn retiretui(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_retiretui"))
        .args(args)
        .output()
        .unwrap()
}

pub fn json_of(output: &Output) -> serde_json::Value {
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

/// A fresh scratch directory `retiretui-{name}` with an empty `nested`
/// directory, the full fixture copied in as `plan`, and the given extra
/// files.
pub fn scratch_dir(name: &str, plan: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("retiretui-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("nested")).unwrap();
    std::fs::copy(FULL_PLAN, dir.join(plan)).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    dir
}
