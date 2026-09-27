//! A year's recorded actions and warnings in words, as every surface says
//! them.

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{Action, ContributionNote, Projection, YearRow, irmaa_purchase};
use retiretui_engine::tax;

use crate::table::{account_name, income_name, money, rate};

/// An action as a sentence, its amount nominal and accounts by their
/// display names.
#[must_use]
pub fn sentence(plan: &Plan, action: &Action) -> String {
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
#[must_use]
pub fn note_phrase(plan: &Plan, note: &ContributionNote) -> String {
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
pub enum Held {
    /// Held to a legal or plan limit.
    ToLimit,
    /// Phased out by MAGI.
    PhasedOut,
    /// Paid, but not all of it deducted.
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
pub fn held_contributions(row: &YearRow) -> impl Iterator<Item = (&str, Held)> {
    row.actions.iter().filter_map(|action| {
        let Action::Contribution { account, notes, .. } = action else {
            return None;
        };
        Some((account.as_str(), notes.iter().find_map(Held::of)?))
    })
}

/// The year's warnings, each amount in the dollars of the year it is paid
/// in, or in today's through `deflating`'s deflators where one is given.
#[must_use]
pub fn collect_warnings(
    plan: &Plan,
    tables: &TaxTables,
    row: &YearRow,
    deflating: Option<&Projection>,
) -> Vec<String> {
    let dollars = |year, amount| deflating.map_or(amount, |years| years.deflate_in(year, amount));
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
