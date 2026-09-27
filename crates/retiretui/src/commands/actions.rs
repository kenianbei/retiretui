use retiretui_client::replies::ActionsReply;
use retiretui_client::replies::year_row;
use retiretui_client::session::Today;
use std::path::PathBuf;

use clap::Args;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{YearRow, project};

use super::project::OutputFormat;
use retiretui_client::actions::{collect_warnings, sentence};

/// Arguments of the `actions` subcommand.
#[derive(Args)]
pub struct ActionsArgs {
    /// Path to the plan TOML file.
    pub plan: PathBuf,
    /// The year to report; defaults to the current calendar year.
    #[arg(long)]
    pub year: Option<i16>,
    /// Output format. Amounts are always nominal - they are instructions.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

pub fn run(args: &ActionsArgs) -> anyhow::Result<()> {
    let tables = super::load_tables(&args.tax_dir)?;
    let plan = super::load_validated_plan(&args.plan, &tables)?;
    let projection = project(&plan, &tables);
    let year = args.year.unwrap_or_else(|| Today::now().0);
    let row = year_row(&projection, year).map_err(anyhow::Error::msg)?;
    let warnings = collect_warnings(&plan, &tables, row, None);
    match args.format {
        OutputFormat::Json => {
            let reply = ActionsReply::new(row, warnings);
            println!("{}", serde_json::to_string_pretty(&reply)?);
        }
        OutputFormat::Table => print!("{}", render(&plan, row, &warnings)),
    }
    Ok(())
}

fn render(plan: &Plan, row: &YearRow, warnings: &[String]) -> String {
    use std::fmt::Write;
    let mut out = format!(
        "Actions for {} (ages {})\n\n",
        row.year,
        retiretui_client::table::ages_text(plan, row)
    );
    if row.actions.is_empty() {
        out.push_str("Nothing scheduled.\n");
    } else {
        for action in &row.actions {
            let _ = writeln!(out, "{}", sentence(plan, action));
        }
    }
    if !warnings.is_empty() {
        out.push('\n');
    }
    for warning in warnings {
        let _ = writeln!(out, "! {warning}");
    }
    out
}
