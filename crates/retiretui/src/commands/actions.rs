use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Args;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{Action, Projection, YearRow, project};
use schemars::JsonSchema;
use serde::Serialize;

use super::project::OutputFormat;
use retiretui_tui::actions::{collect_warnings, sentence};

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

/// One year's to-dos, as `actions` and `plan_actions` reply.
#[derive(Serialize, JsonSchema)]
pub struct ActionsReply {
    /// The reported year.
    pub year: i16,
    /// Age each person reaches during the year, by person id.
    pub ages: BTreeMap<String, u8>,
    /// Executed instructions in execution order, each tagged by `kind`
    /// (`transfer`, `rmd`, `contribution`, `conversion`, `withdrawal`)
    /// with account ids and nominal amounts - they are instructions, so
    /// no deflated variant exists.
    pub actions: Vec<Action>,
    /// Human-readable warnings: unfunded spending, medicare and cliff
    /// costs, and the IRMAA surcharge this year's MAGI buys two years out.
    pub warnings: Vec<String>,
}

impl ActionsReply {
    pub fn new(row: &YearRow, warnings: Vec<String>) -> Self {
        Self {
            year: row.year,
            ages: row.ages.clone(),
            actions: row.actions.clone(),
            warnings,
        }
    }
}

pub fn run(args: &ActionsArgs) -> anyhow::Result<()> {
    let tables = super::load_tables(&args.tax_dir)?;
    let plan = super::load_validated_plan(&args.plan, &tables)?;
    let projection = project(&plan, &tables);
    let year = args.year.unwrap_or_else(current_year);
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

pub(crate) fn current_year() -> i16 {
    jiff::Zoned::now().date().year()
}

/// The `year` row; an out-of-range year errors with the valid range.
pub(crate) fn year_row(projection: &Projection, year: i16) -> Result<&YearRow, String> {
    projection.row(year).ok_or_else(|| {
        let first = projection.years.first().map_or(year, |row| row.year);
        let last = projection.years.last().map_or(year, |row| row.year);
        format!("{year} is outside the projection; the plan covers {first}-{last}")
    })
}

fn render(plan: &Plan, row: &YearRow, warnings: &[String]) -> String {
    use std::fmt::Write;
    let mut out = format!(
        "Actions for {} (ages {})\n\n",
        row.year,
        retiretui_tui::table::ages_text(plan, row)
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
