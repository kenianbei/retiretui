use std::path::{Path, PathBuf};

use clap::Args;
use retiretui_client::environment::{load_history, load_tables};
use retiretui_client::replies::SpendingReply;
use retiretui_client::searches::run_refusal;
use retiretui_client::searches::spending::{DEFAULT_TARGET, Found, search};
use retiretui_client::store::DiskStore;
use retiretui_client::table::{align, plain_dollars, rate, summary_table};
use retiretui_engine::market::Progress;
use retiretui_engine::optimize::spending_overlay;
use retiretui_engine::plan::{Item, Plan};

use crate::project::OutputFormat;

/// Arguments of `optimize spending`.
#[derive(Args, Debug)]
pub struct SpendingArgs {
    /// Path to the plan or scenario TOML file.
    pub plan: PathBuf,
    /// The share of random markets the spending must last in, as a
    /// fraction.
    #[arg(long, default_value_t = DEFAULT_TARGET)]
    pub success: f64,
    /// Write the ceiling at that share as a scenario overlay file.
    #[arg(long)]
    pub write: Option<PathBuf>,
    /// Show future (nominal) dollars instead of today's dollars.
    #[arg(long)]
    pub nominal: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
    /// A historical record of the embedded one's shape to run in its place.
    #[arg(long)]
    pub history: Option<PathBuf>,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

pub fn run(args: &SpendingArgs) -> anyhow::Result<()> {
    let tables = load_tables(&args.tax_dir)?;
    let plan = crate::load_validated_plan(&args.plan, &tables)?;
    let history = load_history(args.history.as_deref())?;
    let found = search(&plan, &tables, &history, args.success, &Progress::default())
        .map_err(|error| anyhow::Error::msg(run_refusal(error)))?;
    let deflated = !args.nominal;
    match args.format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&SpendingReply::new(&found, &plan, deflated))?
        ),
        OutputFormat::Table => print!("{}", spending_text(&plan, &found, deflated)),
    }
    if let Some(out) = &args.write {
        write_at_target(out, &args.plan, &found)?;
        println!("wrote {}", out.display());
    }
    Ok(())
}

/// The plan and each ceiling a row, over each flexible expense now and at
/// each ceiling, in today's dollars a year.
fn spending_text(plan: &Plan, found: &Found, deflated: bool) -> String {
    let listed = found.listed();
    let baseline = vec![
        "baseline".to_owned(),
        plain_dollars(plan.flexible_spending()),
        String::new(),
        rate(found.plan_success()),
    ];
    let mut rows = vec![(
        baseline,
        found.planned.baseline.projection.summary(deflated),
    )];
    rows.extend(listed.iter().map(|listed| {
        let cells = vec![
            listed.held_to.to_lowercase(),
            listed.flexible_said(plain_dollars(listed.flexible())),
            listed.change(plan),
            rate(listed.success),
        ];
        (cells, listed.ceiling.judged.projection.summary(deflated))
    }));
    let ceilings = summary_table(&["held to", "flexible", "change", "success"], rows);

    let header = ["expense", "now"].into_iter();
    let header: Vec<String> = header
        .map(str::to_owned)
        .chain(listed.iter().map(|listed| listed.held_to.to_lowercase()))
        .collect();
    let planned = found.planned.expenses.iter();
    let expenses: Vec<Vec<String>> = planned
        .filter_map(|scaled| {
            let stated = plan.expenses.iter().find(|it| it.id == scaled.id)?;
            let mut at_target = found.at_target.expenses.iter();
            let at_target = at_target.find(|it| it.id == scaled.id)?;
            let amounts = [stated.amount, scaled.amount, at_target.amount];
            let name = stated.display_name().to_owned();
            Some(
                std::iter::once(name)
                    .chain(amounts.map(plain_dollars))
                    .collect(),
            )
        })
        .collect();
    format!("{ceilings}\n{}", align(&header, &expenses))
}

fn write_at_target(out: &Path, plan_path: &Path, found: &Found) -> anyhow::Result<()> {
    let base = retiretui_client::files::overlay_base(&DiskStore, out, plan_path)?;
    let overlay = spending_overlay(&base, &found.at_target.expenses)?;
    retiretui_client::files::write_atomic(out, &overlay)?;
    Ok(())
}
