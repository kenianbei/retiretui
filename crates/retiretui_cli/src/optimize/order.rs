use std::path::{Path, PathBuf};

use clap::Args;
use retiretui_client::replies::OrderReply;
use retiretui_client::searches::run_refusal;
use retiretui_client::store::DiskStore;
use retiretui_client::table::summary_table;
use retiretui_engine::market::Progress;
use retiretui_engine::optimize::{OrderSearch, optimize_order, order_overlay};
use retiretui_engine::plan::{Plan, TreatmentClass};

use crate::project::OutputFormat;

/// Arguments of `optimize order`.
#[derive(Args, Debug)]
pub struct OrderArgs {
    /// Path to the plan or scenario TOML file.
    pub plan: PathBuf,
    /// Write the best-ranked order as a scenario overlay file.
    #[arg(long)]
    pub write: Option<PathBuf>,
    /// Show future (nominal) dollars instead of today's dollars.
    #[arg(long)]
    pub nominal: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

pub fn run(args: &OrderArgs) -> anyhow::Result<()> {
    let tables = retiretui_client::environment::load_tables(&args.tax_dir)?;
    let plan = crate::load_validated_plan(&args.plan, &tables)?;
    let search = optimize_order(&plan, &tables, &Progress::default())
        .map_err(|error| anyhow::Error::msg(run_refusal(error)))?;
    let deflated = !args.nominal;
    match args.format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&OrderReply::new(&search, deflated))?
        ),
        OutputFormat::Table => print!("{}", order_table(&plan, &search, deflated)),
    }
    if let Some(out) = &args.write {
        write_best(out, &args.plan, &search)?;
        println!("wrote {}", out.display());
    }
    Ok(())
}

/// An order as a plan file spells its classes.
fn spelled(order: &[TreatmentClass]) -> String {
    let classes: Vec<&str> = order.iter().map(|class| class.as_str()).collect();
    classes.join(", ")
}

fn order_table(plan: &Plan, search: &OrderSearch, deflated: bool) -> String {
    let baseline = vec!["baseline".to_owned(), spelled(&plan.plan.withdrawal_order)];
    let mut rows = vec![(baseline, search.baseline.summary(deflated))];
    rows.extend(
        search
            .candidates
            .iter()
            .enumerate()
            .map(|(rank, candidate)| {
                let cells = vec![(rank + 1).to_string(), spelled(&candidate.order)];
                (cells, candidate.projection.summary(deflated))
            }),
    );
    summary_table(&["rank", "order"], rows)
}

fn write_best(out: &Path, plan_path: &Path, search: &OrderSearch) -> anyhow::Result<()> {
    let base = retiretui_client::files::overlay_base(&DiskStore, out, plan_path)?;
    let overlay = order_overlay(&base, &search.best().order)?;
    retiretui_client::files::write_atomic(out, &overlay)?;
    Ok(())
}
