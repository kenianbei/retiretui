use std::path::PathBuf;

use clap::{Args, ValueEnum};
use retiretui_engine::plan::Dollars;
use retiretui_engine::project::{Projection, Summary, YearRow, project};
use serde::Serialize;

use super::project::OutputFormat;
use super::table::{align, display_dollars, summary_table};

/// Arguments of the `compare` subcommand.
#[derive(Args)]
pub struct CompareArgs {
    /// Paths to two or more plan or scenario TOML files.
    #[arg(num_args = 2..)]
    pub plans: Vec<PathBuf>,
    /// Compare one metric year by year instead of the summary table
    /// (table output only).
    #[arg(long, value_enum)]
    pub metric: Option<Metric>,
    /// Output format. JSON always carries each plan's summary and its full
    /// nominal year rows with per-year deflators.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
    /// Show future (nominal) dollars instead of today's dollars.
    #[arg(long)]
    pub nominal: bool,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

/// A projected quantity comparable year by year.
#[derive(Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum Metric {
    /// Sum of all end-of-year balances.
    #[default]
    NetWorth,
    /// Gross income, Social Security included.
    Income,
    /// Spending for the year.
    Expenses,
    /// Total taxes assessed.
    Taxes,
    /// Money withdrawn, RMDs included.
    Withdrawals,
    /// Roth conversions executed.
    Conversions,
    /// Modified adjusted gross income.
    Magi,
    /// Spending the accounts could not cover.
    Unfunded,
}

impl Metric {
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::NetWorth => "Net worth",
            Self::Income => "Income",
            Self::Expenses => "Expenses",
            Self::Taxes => "Taxes",
            Self::Withdrawals => "Withdrawals",
            Self::Conversions => "Conversions",
            Self::Magi => "MAGI",
            Self::Unfunded => "Unfunded",
        }
    }

    /// The metric `step` places along, wrapping at either end.
    pub(crate) fn neighbor(self, step: isize) -> Self {
        let all = Self::value_variants();
        let at = all.iter().position(|metric| *metric == self).unwrap_or(0);
        let count = all.len() as isize;
        all[(at as isize + step).rem_euclid(count) as usize]
    }

    pub(crate) fn value(self, row: &YearRow) -> Dollars {
        match self {
            Self::NetWorth => row.net_worth,
            Self::Income => row.total_income,
            Self::Expenses => row.expenses,
            Self::Taxes => row.taxes.total,
            Self::Withdrawals => row.total_withdrawals(),
            Self::Conversions => row.conversions,
            Self::Magi => row.taxes.magi,
            Self::Unfunded => row.unfunded,
        }
    }
}

struct ComparedPlan {
    path: PathBuf,
    name: Option<String>,
    projection: Projection,
}

impl ComparedPlan {
    /// The display name, falling back to the file stem.
    fn label(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            self.path.file_stem().map_or_else(
                || self.path.display().to_string(),
                |stem| stem.to_string_lossy().into_owned(),
            )
        })
    }
}

pub fn run(args: &CompareArgs) -> anyhow::Result<()> {
    let tables = super::load_tables(&args.tax_dir)?;
    let mut compared = Vec::with_capacity(args.plans.len());
    for path in &args.plans {
        let plan = super::load_validated_plan(path, &tables)?;
        compared.push(ComparedPlan {
            projection: project(&plan, &tables),
            name: plan.plan.name,
            path: path.clone(),
        });
    }
    match args.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&reply(&compared, args))?);
        }
        OutputFormat::Table => match args.metric {
            Some(metric) => print!("{}", metric_table(&compared, metric, args.nominal)),
            None => print!("{}", compared_table(&compared, args.nominal)),
        },
    }
    Ok(())
}

fn compared_table(compared: &[ComparedPlan], nominal: bool) -> String {
    let rows = compared
        .iter()
        .map(|plan| (vec![plan.label()], plan.projection.summary(!nominal)))
        .collect();
    summary_table(&["plan"], rows)
}

fn metric_table(compared: &[ComparedPlan], metric: Metric, nominal: bool) -> String {
    let mut header = vec!["year".to_owned()];
    header.extend(compared.iter().map(ComparedPlan::label));
    let first = compared
        .iter()
        .filter_map(|plan| plan.projection.years.first().map(|row| row.year))
        .min();
    let last = compared
        .iter()
        .filter_map(|plan| plan.projection.years.last().map(|row| row.year))
        .max();
    let (Some(first), Some(last)) = (first, last) else {
        return align(&header, &[]);
    };
    let rows: Vec<Vec<String>> = (first..=last)
        .map(|year| {
            let mut cells = vec![year.to_string()];
            for plan in compared {
                let cell = plan
                    .projection
                    .years
                    .iter()
                    .find(|row| row.year == year)
                    .map_or_else(String::new, |row| {
                        display_dollars(metric.value(row), row.deflator, nominal)
                    });
                cells.push(cell);
            }
            cells
        })
        .collect();
    align(&header, &rows)
}

#[derive(Serialize)]
struct CompareReply<'a> {
    plans: Vec<PlanReply<'a>>,
}

#[derive(Serialize)]
struct PlanReply<'a> {
    path: String,
    name: Option<&'a str>,
    summary: Summary,
    years: &'a [YearRow],
}

fn reply<'a>(compared: &'a [ComparedPlan], args: &CompareArgs) -> CompareReply<'a> {
    CompareReply {
        plans: compared
            .iter()
            .map(|plan| PlanReply {
                path: plan.path.display().to_string(),
                name: plan.name.as_deref(),
                summary: plan.projection.summary(!args.nominal),
                years: &plan.projection.years,
            })
            .collect(),
    }
}
