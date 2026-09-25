use std::path::PathBuf;

use clap::{Args, ValueEnum};
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{Projection, YearRow, project};

use super::table::{Column, align, display_dollars, year_figures};

/// Arguments of the `project` subcommand.
#[derive(Args)]
pub struct ProjectArgs {
    /// Path to the plan TOML file.
    pub plan: PathBuf,
    /// Output format. JSON always carries nominal amounts, per-account
    /// detail, and each year's deflator.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
    /// Show future (nominal) dollars instead of today's dollars.
    #[arg(long)]
    pub nominal: bool,
    /// One balance column per account instead of treatment-class totals.
    #[arg(long)]
    pub by_account: bool,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

/// How `project` prints the ledger.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// A year-by-year text table.
    Table,
    /// The full projection as JSON.
    Json,
}

pub fn run(args: &ProjectArgs) -> anyhow::Result<()> {
    let tables = super::load_tables(&args.tax_dir)?;
    let plan = super::load_validated_plan(&args.plan, &tables)?;
    let projection = project(&plan, &tables);
    match args.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&projection)?),
        OutputFormat::Table => print!("{}", render_table(&plan, &projection, args)),
    }
    Ok(())
}

fn render_table(plan: &Plan, projection: &Projection, args: &ProjectArgs) -> String {
    let columns = balance_columns(plan, args.by_account);
    let mut header = vec![
        "year".to_owned(),
        "age".to_owned(),
        "income".to_owned(),
        "expense".to_owned(),
        "tax".to_owned(),
        "wd".to_owned(),
    ];
    header.extend(columns.iter().map(|column| match column {
        Column::Account(id) => id.clone(),
        Column::Class(class) => class.as_str().to_owned(),
    }));
    header.push("net".to_owned());
    let rows: Vec<Vec<String>> = projection
        .years
        .iter()
        .map(|row| format_row(plan, row, &columns, args.nominal))
        .collect();
    align(&header, &rows)
}

fn balance_columns(plan: &Plan, by_account: bool) -> Vec<Column> {
    if by_account {
        return plan
            .accounts
            .iter()
            .map(|account| Column::Account(account.id.clone()))
            .collect();
    }
    super::table::present_classes(plan)
        .into_iter()
        .map(Column::Class)
        .collect()
}

fn format_row(plan: &Plan, row: &YearRow, columns: &[Column], nominal: bool) -> Vec<String> {
    let mut cells = vec![row.year.to_string(), super::table::ages_text(plan, row)];
    cells.extend(
        year_figures(row, columns)
            .into_iter()
            .map(|amount| display_dollars(amount, row.deflator, nominal)),
    );
    cells
}
