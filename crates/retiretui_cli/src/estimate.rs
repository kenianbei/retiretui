use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use retiretui_client::environment::{load_patterns, load_tables};
use retiretui_client::estimate::{self, EstimateReply, Exports, listed};
use retiretui_client::present::MoneyForm;
use retiretui_client::store::DiskStore;
use retiretui_client::table::align;
use retiretui_engine::estimate::Estimate;
use retiretui_engine::plan::Plan;

use crate::project::OutputFormat;

/// Arguments of the `estimate` subcommand.
#[derive(Args, Debug)]
pub struct EstimateArgs {
    /// Path to the plan or scenario TOML file.
    pub plan: PathBuf,
    /// A bank's exports: CSV with a header row, OFX or QFX.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// A `[[patterns]]` file read before the embedded table and the one in
    /// the config directory's `estimate` folder.
    #[arg(long)]
    pub patterns: Option<PathBuf>,
    /// Write the proposals applied as a scenario overlay file.
    #[arg(long)]
    pub write: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

pub fn run(args: &EstimateArgs) -> anyhow::Result<()> {
    let tables = load_tables(&args.tax_dir)?;
    let plan = crate::load_validated_plan(&args.plan, &tables)?;
    let patterns = load_patterns(args.patterns.as_deref())?;
    let mut exports = Exports::default();
    for path in &args.files {
        let text = fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("failed to read {}: {error}", path.display()))?;
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        exports.add(&name, &text).map_err(anyhow::Error::msg)?;
    }
    let found = exports.estimate(&plan, &patterns);
    let written = match &args.write {
        Some(out) => {
            write_applied(out, &args.plan, &plan, &found)?;
            Some(out.display().to_string())
        }
        None => None,
    };
    match args.format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&EstimateReply {
                estimate: found,
                written
            })?
        ),
        OutputFormat::Table => {
            print!("{}", estimate_text(&found));
            if let Some(out) = written {
                println!("wrote {out}");
            }
        }
    }
    Ok(())
}

/// Each expense with what it gathers beneath it, then each payee nothing
/// names, under the summary line; the first column left-aligned so the
/// nesting shows.
fn estimate_text(found: &Estimate) -> String {
    let header: Vec<String> = estimate::COLUMNS.map(str::to_lowercase).to_vec();
    let mut rows: Vec<Vec<String>> = listed(found)
        .iter()
        .map(|row| row.cells(found.months, MoneyForm::Full).to_vec())
        .collect();
    let width = rows
        .iter()
        .map(|row| row[0].len())
        .chain([header[0].len()])
        .max()
        .unwrap_or_default();
    for row in &mut rows {
        row[0] = format!("{:<width$}", row[0]);
    }
    let mut header = header;
    header[0] = format!("{:<width$}", header[0]);
    format!(
        "{}\n{}",
        estimate::summary(found, MoneyForm::Full),
        align(&header, &rows)
    )
}

fn write_applied(
    out: &Path,
    plan_path: &Path,
    plan: &Plan,
    found: &Estimate,
) -> anyhow::Result<()> {
    let base = retiretui_client::files::overlay_base(&DiskStore, out, plan_path)?;
    let overlay = estimate::overlay(&base, plan, found).map_err(anyhow::Error::msg)?;
    retiretui_client::files::write_atomic(out, &overlay)?;
    Ok(())
}
