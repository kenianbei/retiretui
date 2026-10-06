//! A year's tax: what was paid of each kind, the bracket its taxable
//! income reaches, and the figures the tax was worked out from.

use retiretui_engine::params::{Bracket, Inflation, TaxTables};
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::YearRow;

use super::DetailLine;
use super::funds::{TOTAL, line, paid};
use crate::table::rate;

/// What the Ledger titles a year's tax.
pub const TAX: &str = "Tax";
const PERCENT: f64 = 100.0;
/// What the bracket no bracket is above is called.
const TOP_BRACKET: &str = "Top bracket";

/// `amounts` of `row` that are more than nothing, said.
fn lines<'a>(
    row: &YearRow,
    is_nominal: bool,
    amounts: impl IntoIterator<Item = (&'a str, Dollars)>,
) -> Vec<DetailLine> {
    let said = paid(amounts).into_iter();
    said.map(|(label, amount)| line(row, is_nominal, &label, amount))
        .collect()
}

/// What `row` paid of each kind of tax, a line only where it paid any, and
/// then all of it.
pub(super) fn tax_lines(row: &YearRow, is_nominal: bool) -> Vec<DetailLine> {
    let taxes = &row.taxes;
    let kinds = [
        ("Ordinary", taxes.ordinary),
        ("State", taxes.state),
        ("Capital gains", taxes.ltcg),
        ("Penalties", taxes.penalty),
    ];
    let mut said = lines(row, is_nominal, kinds);
    said.push(line(row, is_nominal, TOTAL, taxes.total));
    said
}

/// The bracket `taxable` reaches among `brackets`, and the next one's
/// start where there is one.
fn reached(brackets: &[Bracket], taxable: Dollars) -> Option<(&Bracket, Option<Dollars>)> {
    let at = brackets.iter().rposition(|each| each.over <= taxable)?;
    let above = brackets.get(at + 1).map(|next| next.over);
    Some((&brackets[at], above))
}

/// The room left under the top of the federal bracket `row`'s taxable
/// income reaches - or, of the last bracket, its rate - from the tables the
/// plan's own projection applies: the latest grown at the plan's
/// inflation, for its filing status.
pub(super) fn bracket(
    plan: &Plan,
    tables: &TaxTables,
    row: &YearRow,
    is_nominal: bool,
) -> Option<DetailLine> {
    let params = tables.params_for(row.year, &Inflation::constant(plan.plan.inflation));
    let brackets = params.brackets.for_status(plan.household.filing);
    let taxable = row.taxes.ordinary_taxable;
    let (bracket, above) = reached(brackets, taxable)?;
    let rate = rate(bracket.rate);
    Some(match above {
        Some(above) => line(
            row,
            is_nominal,
            &format!("To top of {rate}"),
            above - taxable,
        ),
        None => DetailLine {
            label: TOP_BRACKET.to_owned(),
            amount: rate,
        },
    })
}

/// What `row`'s tax was worked out from, a line only where there is any,
/// and its tax as a share of its MAGI where it has one.
pub(super) fn picture(row: &YearRow, is_nominal: bool) -> Vec<DetailLine> {
    let taxes = &row.taxes;
    let figures = [
        ("MAGI", taxes.magi),
        ("Taxable income", taxes.ordinary_taxable),
        ("Realized gains", taxes.gains),
        ("Taxable Soc. Sec.", taxes.taxable_social_security),
    ];
    let mut said = lines(row, is_nominal, figures);
    if taxes.magi > 0 {
        let share = taxes.total as f64 / taxes.magi as f64 * PERCENT;
        said.push(DetailLine {
            label: "Tax over MAGI".to_owned(),
            amount: format!("{share:.1}%"),
        });
    }
    said
}
