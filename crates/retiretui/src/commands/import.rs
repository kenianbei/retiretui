use std::fs;
use std::path::PathBuf;

use clap::Args;
use retiretui_engine::project::validate_plan;

/// Arguments of the `import-earnings` subcommand.
#[derive(Args)]
pub struct ImportEarningsArgs {
    /// Path to the plan TOML file; rewritten in canonical form.
    pub plan: PathBuf,
    /// The Social Security statement, as the XML downloaded from ssa.gov.
    pub statement: PathBuf,
    /// The person the statement is for.
    #[arg(long)]
    pub person: String,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

/// Pulls the statement's earnings record onto the person and writes the
/// plan back, once; the statement is not referred to again.
pub fn run(args: &ImportEarningsArgs) -> anyhow::Result<()> {
    let read = |path: &PathBuf| {
        fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("failed to read {}: {error}", path.display()))
    };
    let (text, xml) = (read(&args.plan)?, read(&args.statement)?);
    let plan = super::adopt_statement(&text, &args.person, &xml)
        .map_err(|reason| anyhow::anyhow!("{}: {reason}", args.plan.display()))?;
    let tables = super::load_tables(&args.tax_dir)?;
    let issues = validate_plan(&plan, &tables);
    if !issues.is_empty() {
        anyhow::bail!(
            "not written, {} issue(s):\n{}",
            issues.len(),
            super::issue_listing(&issues)
        );
    }
    super::write_plan(&args.plan, &plan).map_err(anyhow::Error::msg)?;
    let recorded = &plan
        .person(&args.person)
        .map(|person| &person.earnings)
        .ok_or_else(|| anyhow::anyhow!("unknown person `{}`", args.person))?;
    let span = match recorded.keys().min().zip(recorded.keys().max()) {
        Some((first, last)) => format!("{first}-{last}"),
        None => "none".to_owned(),
    };
    println!(
        "{}: {} year(s) of earnings ({span}) recorded for {}",
        args.plan.display(),
        recorded.len(),
        args.person
    );
    Ok(())
}
