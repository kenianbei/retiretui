use std::path::{Path, PathBuf};

use clap::Args;
use retiretui_engine::optimize::{Claim, ClaimSearch, claims_overlay, optimize_claims};
use retiretui_engine::project::Summary;
use schemars::JsonSchema;
use serde::Serialize;

use crate::commands::project::OutputFormat;
use crate::commands::table::summary_table;

/// Arguments of `optimize claims`.
#[derive(Args)]
pub struct ClaimArgs {
    /// Path to the plan or scenario TOML file.
    pub plan: PathBuf,
    /// Id of a social-security income to search (repeatable); absent
    /// searches every income whose benefit is computed, and makes one up
    /// for each person with an earnings record and none.
    #[arg(long)]
    pub income: Vec<String>,
    /// Write the best-ranked claims as a scenario overlay file.
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

pub fn run(args: &ClaimArgs) -> anyhow::Result<()> {
    let tables = crate::commands::load_tables(&args.tax_dir)?;
    let plan = crate::commands::load_validated_plan(&args.plan, &tables)?;
    let search = optimize_claims(&plan, &tables, &args.income, &[])
        .map_err(|issues| anyhow::Error::msg(crate::commands::issue_listing(&issues)))?;
    let deflated = !args.nominal;
    match args.format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&ClaimsReply::new(&search, deflated))?
        ),
        OutputFormat::Table => print!("{}", claims_table(&search, deflated)),
    }
    if let Some(out) = &args.write {
        write_best(out, &args.plan, &search)?;
        println!("wrote {}", out.display());
    }
    Ok(())
}

/// A claim search, as `optimize claims` and `optimize_claims` reply.
#[derive(Serialize, JsonSchema)]
pub struct ClaimsReply {
    /// Headline figures with the plan's own claims.
    pub baseline: Summary,
    /// The income ids searched, in the order each candidate's claims hold
    /// them.
    pub incomes: Vec<String>,
    /// Every candidate, best first: least unfunded spending, then the
    /// highest final net worth in today's dollars, then earlier claims.
    pub candidates: Vec<ClaimEntry>,
}

/// One set of claims the search tried.
#[derive(Serialize, JsonSchema)]
pub struct ClaimEntry {
    /// One claim per searched income.
    pub claims: Vec<Claim>,
    /// Headline figures under those claims.
    pub summary: Summary,
}

impl ClaimsReply {
    pub fn new(search: &ClaimSearch, deflated: bool) -> Self {
        Self {
            baseline: search.baseline.summary(deflated),
            incomes: search.incomes.clone(),
            candidates: search
                .candidates
                .iter()
                .map(|candidate| ClaimEntry {
                    claims: candidate.claims.clone(),
                    summary: candidate.projection.summary(deflated),
                })
                .collect(),
        }
    }
}

/// The headings `rank` and each searched income, then the baseline with
/// `-` for each and a row per candidate with its rank and claim ages,
/// ahead of the summary figures.
fn claims_table(search: &ClaimSearch, deflated: bool) -> String {
    let mut leading = vec!["rank"];
    leading.extend(search.incomes.iter().map(String::as_str));
    let mut baseline = vec!["baseline".to_owned()];
    baseline.extend(search.incomes.iter().map(|_| "-".to_owned()));
    let mut rows = vec![(baseline, search.baseline.summary(deflated))];
    rows.extend(
        search
            .candidates
            .iter()
            .enumerate()
            .map(|(rank, candidate)| {
                let mut cells = vec![(rank + 1).to_string()];
                cells.extend(candidate.claims.iter().map(|claim| claim.age.to_string()));
                (cells, candidate.projection.summary(deflated))
            }),
    );
    summary_table(&leading, rows)
}

fn write_best(out: &Path, plan_path: &Path, search: &ClaimSearch) -> anyhow::Result<()> {
    let base = crate::commands::overlay_base(out, plan_path)?;
    let overlay = claims_overlay(&base, &search.added, &search.best().claims)?;
    crate::commands::write_atomic(out, &overlay)?;
    Ok(())
}
