use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Args;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::{
    Action, ContributionNote, Projection, YearRow, irmaa_purchase, project,
};
use retiretui_engine::tax;
use schemars::JsonSchema;
use serde::Serialize;

use super::project::OutputFormat;
use super::table::{account_name, income_name, money, rate};

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
    let warnings = collect_warnings(&plan, &tables, row, nominal);
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
    projection
        .years
        .iter()
        .find(|row| row.year == year)
        .ok_or_else(|| {
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
        super::table::ages_text(plan, row)
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

/// An action as a sentence, its amount nominal and accounts by their
/// display names.
pub(crate) fn sentence(plan: &Plan, action: &Action) -> String {
    let named = |id: &str| account_name(plan, id).to_owned();
    match action {
        Action::Transfer { from, to, amount } => {
            format!(
                "Transfer {} from {} to {}",
                money(*amount),
                named(from),
                named(to)
            )
        }
        Action::Rmd { account, amount } => format!(
            "Take the required distribution of {} from {}",
            money(*amount),
            named(account)
        ),
        Action::Contribution {
            account,
            employee,
            employer,
            notes,
        } => {
            let total = money(employee + employer);
            let account = named(account);
            let mut said = Vec::new();
            if *employer > 0 {
                let (yours, theirs) = (money(*employee), money(*employer));
                said.push(format!("{yours} yours, {theirs} employer"));
            }
            said.extend(notes.iter().map(|note| note_phrase(plan, note)));
            if said.is_empty() {
                format!("Contribute {total} to {account}")
            } else {
                format!("Contribute {total} to {account} ({})", said.join("; "))
            }
        }
        Action::Conversion { from, to, amount } => {
            format!(
                "Convert {} from {} to {}",
                money(*amount),
                named(from),
                named(to)
            )
        }
        Action::Withdrawal { account, amount } => {
            format!("Withdraw {} from {}", money(*amount), named(account))
        }
        Action::Surplus { account, amount } => {
            format!("Save the unspent {} in {}", money(*amount), named(account))
        }
    }
}

/// How a contribution came to be what it is, as the sentence says it.
pub(crate) fn note_phrase(plan: &Plan, note: &ContributionNote) -> String {
    match note {
        ContributionNote::Share { rate: share, of } => {
            format!("{} of {}", rate(*share), income_name(plan, of))
        }
        ContributionNote::Maximum => "the maximum".to_owned(),
        ContributionNote::Match {
            rate: share,
            up_to,
            of,
        } => format!(
            "{} match up to {} of {}",
            rate(*share),
            rate(*up_to),
            income_name(plan, of)
        ),
        ContributionNote::AfterTax { amount } => format!("{} after tax", money(*amount)),
        ContributionNote::HeldToLimit => "held to the limit".to_owned(),
        ContributionNote::HeldToOverall => "employer share held to the plan's cap".to_owned(),
        ContributionNote::RothIraPhaseOut => "over the Roth IRA income limit".to_owned(),
        ContributionNote::NotDeducted { amount } => format!("{} not deductible", money(*amount)),
    }
}

/// How a year could not take a contribution as stated.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Held {
    ToLimit,
    PhasedOut,
    NotDeducted,
}

impl Held {
    const fn of(note: &ContributionNote) -> Option<Self> {
        match note {
            ContributionNote::HeldToLimit | ContributionNote::HeldToOverall => Some(Self::ToLimit),
            ContributionNote::RothIraPhaseOut => Some(Self::PhasedOut),
            ContributionNote::NotDeducted { .. } => Some(Self::NotDeducted),
            ContributionNote::Share { .. }
            | ContributionNote::Maximum
            | ContributionNote::Match { .. }
            | ContributionNote::AfterTax { .. } => None,
        }
    }

    const fn warning(self) -> &'static str {
        match self {
            Self::ToLimit => "contributions were held to a limit this year",
            Self::PhasedOut => "this year's MAGI is over the Roth IRA contribution limit",
            Self::NotDeducted => "part of the IRA contribution is not deductible this year",
        }
    }
}

/// The accounts whose contributions a year could not take as stated,
/// each beside how.
pub(crate) fn held_contributions(row: &YearRow) -> impl Iterator<Item = (&str, Held)> {
    row.actions.iter().filter_map(|action| {
        let Action::Contribution { account, notes, .. } = action else {
            return None;
        };
        Some((account.as_str(), notes.iter().find_map(Held::of)?))
    })
}

/// Keeps an amount in the dollars of the year it is paid in.
pub(crate) fn nominal(_: i16, amount: Dollars) -> Dollars {
    amount
}

/// The year's warnings, each amount read through `dollars` from the
/// dollars of the year it is paid in: `nominal` keeps them so.
pub(crate) fn collect_warnings(
    plan: &Plan,
    tables: &TaxTables,
    row: &YearRow,
    dollars: impl Fn(i16, Dollars) -> Dollars,
) -> Vec<String> {
    let mut warnings: Vec<String> = held_contributions(row)
        .map(|(account, held)| format!("{}: {}", account_name(plan, account), held.warning()))
        .collect();
    if row.unfunded > 0 {
        warnings.push(format!(
            "Unfunded: spending exceeds available money by {}",
            money(dollars(row.year, row.unfunded))
        ));
    }
    if row.medicare > 0 {
        warnings.push(format!(
            "Medicare surcharges and cliff costs paid this year: {}",
            money(dollars(row.year, row.medicare))
        ));
    }
    let purchase = irmaa_purchase(plan, tables, row.year, row.taxes.magi);
    if purchase > 0 {
        let premium_year = row.year + tax::IRMAA_LOOKBACK_YEARS;
        warnings.push(format!(
            "This year's MAGI ({}) buys {} in IRMAA surcharges in {premium_year}",
            money(dollars(row.year, row.taxes.magi)),
            money(dollars(premium_year, purchase)),
        ));
    }
    warnings
}
