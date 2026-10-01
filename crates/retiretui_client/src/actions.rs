//! A year's recorded actions and warnings in words, as every surface says
//! them.

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::{Action, ContributionNote, Projection, YearRow, irmaa_purchase};
use retiretui_engine::tax;

use crate::session::Projected;
use crate::table::{account_name, basis_amount, income_name, money, rate};

/// What a surface says of a year with nothing to do and nothing to watch.
pub const NOTHING_SCHEDULED: &str = "Nothing to do this year.";

/// `row`'s actions and warnings in words, nominal or in today's dollars.
#[must_use]
pub fn year_in_words(
    projected: &Projected,
    tables: &TaxTables,
    row: &YearRow,
    nominal: bool,
) -> (Vec<String>, Vec<String>) {
    let deflator = (!nominal).then_some(row.deflator);
    let deflating = (!nominal).then_some(&projected.projection);
    let actions = row.actions.iter();
    let actions = actions.map(|action| sentence(&projected.plan, action, deflator));
    let warnings = collect_warnings(&projected.plan, tables, row, deflating);
    (actions.collect(), warnings)
}

/// An action as a sentence, accounts by their display names and its
/// amount nominal, or in today's dollars through its year's `deflator`
/// where one is given.
#[must_use]
pub fn sentence(plan: &Plan, action: &Action, deflator: Option<f64>) -> String {
    let named = |id: &str| account_name(plan, id).to_owned();
    let money = |amount| money(deflator.map_or(amount, |by| basis_amount(amount, by, false)));
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
            let said = contribution_notes(plan, (*employee, *employer), notes, &money);
            let (total, account) = (money(employee + employer), named(account));
            if said.is_empty() {
                format!("Contribute {total} to {account}")
            } else {
                format!("Contribute {total} to {account} ({said})")
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

/// What a contribution's sentence adds in brackets: the employee's and
/// employer's shares where both paid, and how it came to be what it is.
fn contribution_notes(
    plan: &Plan,
    (yours, theirs): (Dollars, Dollars),
    notes: &[ContributionNote],
    money: &impl Fn(Dollars) -> String,
) -> String {
    let mut said = Vec::new();
    if theirs > 0 {
        said.push(format!(
            "{} yours, {} employer",
            money(yours),
            money(theirs)
        ));
    }
    said.extend(notes.iter().map(|note| note_phrase(plan, note)));
    said.join("; ")
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
    if row.taxes.penalty > 0 {
        warnings.push(format!(
            "Early-withdrawal penalty paid this year: {}",
            money(dollars(row.year, row.taxes.penalty))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overview::tests::{TEST_PLAN, projected_from, test_projected};

    const PENALTY_PAID: &str = "Early-withdrawal penalty paid this year: ";

    #[test]
    fn a_year_that_pays_the_early_withdrawal_penalty_says_so_in_the_basis_shown() {
        let tables = TaxTables::embedded();
        let early = projected_from(&TEST_PLAN.replace("amount = 60000", "amount = 160000"));
        let row = &early.projection.years[2];
        assert!(
            row.taxes.penalty > 0,
            "a year that draws on the 401(k) at 48"
        );
        let said = |deflating| collect_warnings(&early.plan, &tables, row, deflating);
        let nominal = format!("{PENALTY_PAID}{}", money(row.taxes.penalty));
        assert!(said(None).contains(&nominal), "{:?}", said(None));
        let deflated = early.projection.deflate_in(row.year, row.taxes.penalty);
        assert_ne!(deflated, row.taxes.penalty);
        let todays = format!("{PENALTY_PAID}{}", money(deflated));
        let in_todays = said(Some(&early.projection));
        assert!(in_todays.contains(&todays), "{in_todays:?}");

        let unpenalized = test_projected();
        for row in &unpenalized.projection.years {
            assert_eq!(row.taxes.penalty, 0);
            let said = collect_warnings(&unpenalized.plan, &tables, row, None);
            assert!(!said.iter().any(|line| line.starts_with(PENALTY_PAID)));
        }
    }
}
