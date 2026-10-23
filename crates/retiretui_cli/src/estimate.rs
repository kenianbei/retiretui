use std::fs;
use std::path::{Path, PathBuf};

use clap::{ArgGroup, Args};
use retiretui_client::environment::{load_patterns, load_tables};
use retiretui_client::estimate::{self, EstimateReply, Exports, RecordsReply, listed};
use retiretui_client::present::MoneyForm;
use retiretui_client::store::DiskStore;
use retiretui_client::table::align;
use retiretui_engine::estimate::{Category, Estimate, Of};
use retiretui_engine::plan::Plan;

use crate::project::OutputFormat;

/// The columns records are tabled under.
const RECORD_COLUMNS: [&str; 5] = ["date", "payee", "amount", "sorted", "by"];
/// The record columns that hold words, left-aligned.
const RECORD_TEXT_COLUMNS: [usize; 3] = [1, 3, 4];

/// Arguments of the `estimate` subcommand.
#[derive(Args, Debug)]
#[command(group(ArgGroup::new("records").args(["category", "payee", "unnamed", "ignored"])))]
pub struct EstimateArgs {
    /// Path to the plan or scenario TOML file.
    pub plan: PathBuf,
    /// A bank's exports: CSV with a header row, OFX or QFX.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// A patterns file read before the one in the config directory's
    /// `estimate` folder and the embedded table, winning a tie with either.
    #[arg(long)]
    pub patterns: Option<PathBuf>,
    /// Leave out the patterns file in the config directory's `estimate`
    /// folder.
    #[arg(long)]
    pub no_user_patterns: bool,
    /// Write the proposals applied as a scenario overlay file.
    #[arg(long, conflicts_with = "records")]
    pub write: Option<PathBuf>,
    /// List the records sorted into this category, by its slug, in place of
    /// the estimate.
    #[arg(long, value_parser = parse_category)]
    pub category: Option<Category>,
    /// List the records of every payee containing these words, either case,
    /// by its name or as the bank wrote it, wherever they were sorted.
    #[arg(long)]
    pub payee: Option<String>,
    /// List the records of the payees nothing names.
    #[arg(long)]
    pub unnamed: bool,
    /// List the records the patterns ignore.
    #[arg(long)]
    pub ignored: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

impl EstimateArgs {
    /// The records asked for, if any.
    fn selection(&self) -> Option<Of> {
        if let Some(category) = self.category {
            return Some(Of::Category(category));
        }
        if let Some(words) = &self.payee {
            return Some(Of::Containing(words.clone()));
        }
        (self.unnamed.then_some(Of::Unnamed)).or_else(|| self.ignored.then_some(Of::Ignored))
    }
}

/// A category by its slug.
fn parse_category(slug: &str) -> Result<Category, String> {
    let found = Category::ALL
        .iter()
        .find(|category| category.slug() == slug);
    found.copied().ok_or_else(|| {
        let slugs: Vec<&str> = Category::ALL
            .iter()
            .map(|category| category.slug())
            .collect();
        format!("one of {}", slugs.join(", "))
    })
}

pub fn run(args: &EstimateArgs) -> anyhow::Result<()> {
    let tables = load_tables(&args.tax_dir)?;
    let plan = crate::load_validated_plan(&args.plan, &tables)?;
    let patterns = load_patterns(args.patterns.as_deref(), !args.no_user_patterns)?;
    let mut exports = Exports::default();
    for path in &args.files {
        let text = fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("failed to read {}: {error}", path.display()))?;
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        exports.add(&name, &text).map_err(anyhow::Error::msg)?;
    }
    if let Some(of) = args.selection() {
        let found = exports.records(&plan, &patterns, &of);
        match args.format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&found)?),
            OutputFormat::Table => print!("{}", records_text(&found)),
        }
        return Ok(());
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

/// Each expense with what it gathers beneath it, the bucket's last the
/// payees nothing names, under the summary line; the first column
/// left-aligned so the nesting shows.
fn estimate_text(found: &Estimate) -> String {
    let header = estimate::COLUMNS.map(str::to_lowercase);
    let rows: Vec<Vec<String>> = listed(found)
        .map(|row| row.cells(MoneyForm::Full).to_vec())
        .collect();
    format!(
        "{}\n{}",
        estimate::summary(found, MoneyForm::Full),
        tabled(&header, rows, &[0])
    )
}

/// Each record, its words left-aligned, under the heading that totals
/// them.
fn records_text(found: &RecordsReply) -> String {
    let rows: Vec<Vec<String>> = (found.records.iter())
        .map(|record| record.cells().to_vec())
        .collect();
    let header = RECORD_COLUMNS.map(str::to_owned);
    format!(
        "{}\n{}",
        found.heading(),
        tabled(&header, rows, &RECORD_TEXT_COLUMNS)
    )
}

/// `rows` under `header`, the columns at `left` left-aligned and the rest
/// right.
fn tabled(header: &[String], mut rows: Vec<Vec<String>>, left: &[usize]) -> String {
    let mut header = header.to_vec();
    for &at in left {
        let width = (rows.iter().map(|row| row[at].len()))
            .chain([header[at].len()])
            .max()
            .unwrap_or_default();
        for cells in rows.iter_mut().chain([&mut header]) {
            cells[at] = format!("{:<width$}", cells[at]);
        }
    }
    align(&header, &rows)
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
