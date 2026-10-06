//! What the surfaces draw the Ledger from until they draw its views.

use retiretui_engine::plan::{Dollars, Plan, TreatmentClass};
use retiretui_engine::project::YearRow;

use super::DetailLine;
use crate::present::{income_name, money, treatment_class};
use crate::table::basis_amount;

const TEXT_HEADERS: [&str; 2] = ["Year", "Age"];
const FIGURE_HEADERS: [&str; 4] = ["Income", "Spending", "Tax", "Withdrawn"];
const NET_WORTH: &str = "Net worth";

/// The flows table's headers, each beside whether its column holds figures.
pub const FLOW_HEADERS: [(&str, bool); 6] = [
    ("Account", false),
    ("Open", true),
    ("In", false),
    ("Out", false),
    ("Growth", true),
    ("Close", true),
];
/// The year's income beside what it paid, as every surface titles it.
pub const INCOME_AND_TAX: &str = "Income & Tax";

/// The year table's headers, each beside whether its column holds figures:
/// the year, ages and the figures [`crate::table::year_figures`] gives over
/// `classes`.
#[must_use]
pub fn ledger_headers(classes: &[TreatmentClass]) -> Vec<(&'static str, bool)> {
    let classes = classes.iter().map(|&class| treatment_class(class));
    let figures = FIGURE_HEADERS.into_iter().chain(classes).chain([NET_WORTH]);
    let text = TEXT_HEADERS.into_iter().map(|header| (header, false));
    text.chain(figures.map(|header| (header, true))).collect()
}

/// The year's income by source, and then its spending and what it paid
/// besides - a line only where it paid any - on the basis asked for.
#[must_use]
pub fn income_and_tax(
    plan: &Plan,
    row: &YearRow,
    is_nominal: bool,
) -> (Vec<DetailLine>, Vec<DetailLine>) {
    let line = |label: &str, amount: Dollars| DetailLine {
        label: label.to_owned(),
        amount: money(basis_amount(amount, row.deflator, is_nominal)),
    };
    let income = row
        .income
        .iter()
        .map(|(source, &amount)| line(income_name(plan, source), amount))
        .collect();
    let paid = [
        ("Spending", row.expenses),
        ("Ordinary tax", row.taxes.ordinary),
        ("State tax", row.taxes.state),
        ("Capital gains", row.taxes.ltcg),
        ("Penalties", row.taxes.penalty),
        ("Medicare", row.medicare),
        ("Surplus", row.surplus),
        ("Unfunded", row.unfunded),
    ]
    .into_iter()
    .filter(|&(_, amount)| amount != 0)
    .map(|(label, amount)| line(label, amount))
    .collect();
    (income, paid)
}
