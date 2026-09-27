pub mod actions;
pub mod compare;
pub mod import;
pub mod markets;
pub mod mcp;
pub mod optimize;
pub mod project;
pub mod tui;

use std::path::{Path, PathBuf};

use retiretui_engine::market::RunError;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Issue, Plan};
use retiretui_tui::files::{Invalid, validated_plan_with_files};

pub fn run_validate(path: &Path) -> anyhow::Result<()> {
    let tables = load_tables(&[])?;
    load_validated_plan(path, &tables)?;
    println!("{}: ok", path.display());
    Ok(())
}

/// Loads a plan or scenario file and refuses an invalid one, printing its
/// issues to stderr.
fn load_validated_plan(path: &Path, tables: &TaxTables) -> anyhow::Result<Plan> {
    let invalid = match validated_plan_with_files(path, tables).0 {
        Ok(plan) => return Ok(plan),
        Err(invalid) => invalid,
    };
    match &invalid {
        Invalid::Load(message) => message.lines().skip(1).for_each(|line| eprintln!("{line}")),
        Invalid::Issues { issues, .. } => issues.iter().for_each(|issue| eprintln!("{issue}")),
    }
    Err(anyhow::Error::msg(invalid.headline().to_owned()))
}

/// One issue per line, in validation order.
fn issue_listing(issues: &[Issue]) -> String {
    let listing: Vec<String> = issues.iter().map(ToString::to_string).collect();
    listing.join("\n")
}

/// Why a search answered nothing, as the CLI and MCP say it.
pub(crate) fn run_refusal(error: RunError) -> String {
    match error {
        RunError::Cancelled => "the search was cancelled".to_owned(),
        RunError::Refused(issues) => issue_listing(&issues),
    }
}

/// The plan `text` holds with a statement's earnings recorded on `person`,
/// and the statement's note on any years it spread. A scenario is refused:
/// a resolved plan cannot be written back into an overlay, so the record
/// belongs in its base plan.
pub(crate) fn adopt_statement(
    text: &str,
    person: &str,
    xml: &str,
) -> Result<(Plan, Option<String>), String> {
    use retiretui_engine::plan::Scenario;
    let is_scenario = Scenario::from_toml_str(text)
        .map_err(|error| error.to_string())?
        .is_some();
    if is_scenario {
        return Err("a scenario; import the record into its base plan".to_owned());
    }
    let mut plan = Plan::from_toml_str(text).map_err(|error| error.to_string())?;
    let statement = retiretui_engine::statement::parse(xml).map_err(|error| error.to_string())?;
    plan.adopt_earnings(person, &statement)
        .map_err(|issue| issue.message)?;
    Ok((plan, statement.spread_note()))
}

/// The user's own directory `name` in the application's config directory.
pub(crate) fn user_config_dir(name: &str) -> Option<PathBuf> {
    use etcetera::BaseStrategy;
    let strategy = etcetera::choose_base_strategy().ok()?;
    Some(strategy.config_dir().join("retiretui").join(name))
}

fn load_tables(extra_dirs: &[PathBuf]) -> anyhow::Result<TaxTables> {
    let mut tables = TaxTables::embedded();
    if let Some(dir) = user_config_dir("tax") {
        tables.add_dir(&dir)?;
    }
    for dir in extra_dirs {
        tables.add_dir(dir)?;
    }
    Ok(tables)
}
