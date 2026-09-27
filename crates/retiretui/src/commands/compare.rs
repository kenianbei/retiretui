use std::path::PathBuf;

use clap::Args;
use retiretui_engine::project::{Projection, Summary, YearRow, project};
use serde::Serialize;

use super::project::OutputFormat;
use retiretui_client::metric::Metric;
use retiretui_client::table::{align, display_dollars, summary_table};

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
                let cell = plan.projection.row(year).map_or_else(String::new, |row| {
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
