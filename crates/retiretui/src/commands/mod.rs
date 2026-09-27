pub mod actions;
pub mod compare;
pub mod import;
pub mod markets;
pub mod mcp;
pub mod optimize;
pub mod project;
pub mod tui;

use retiretui_client::environment::load_tables;
use retiretui_client::store::DiskStore;
use std::path::Path;

use retiretui_client::files::{Invalid, validated_plan_with_files};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

pub fn run_validate(path: &Path) -> anyhow::Result<()> {
    let tables = load_tables(&[])?;
    load_validated_plan(path, &tables)?;
    println!("{}: ok", path.display());
    Ok(())
}

/// Loads a plan or scenario file and refuses an invalid one, printing its
/// issues to stderr.
fn load_validated_plan(path: &Path, tables: &TaxTables) -> anyhow::Result<Plan> {
    let invalid = match validated_plan_with_files(&DiskStore, path, tables).0 {
        Ok(plan) => return Ok(plan),
        Err(invalid) => invalid,
    };
    match &invalid {
        Invalid::Load(message) => message.lines().skip(1).for_each(|line| eprintln!("{line}")),
        Invalid::Issues { issues, .. } => issues.iter().for_each(|issue| eprintln!("{issue}")),
    }
    Err(anyhow::Error::msg(invalid.headline().to_owned()))
}
