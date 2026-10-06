//! One year in full: who turns what age, the milestones that fall in it,
//! what to do and what to watch, how far the plan has come, each account's
//! flows, where its money came from and went, and its tax.

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Item, Plan};
use retiretui_engine::project::{Projection, YearRow};
use serde::Serialize;

use super::TO_DO;
use super::flows::{AccountFlows, account_flows, all_accounts};
use super::funds::{DetailLine, Funds, money_in, money_out};
use super::tax::{bracket, picture, tax_lines};
use crate::actions::{NOTHING_SCHEDULED, actions_said, collect_warnings};
use crate::overview::milestones;
use crate::present::{basis_name, compact_money};
use crate::session::Projected;
use crate::table::basis_amount;

const JOIN: &str = " · ";
/// What the Ledger titles how far the plan has come.
const SO_FAR: &str = "So far";

/// Which year is asked for, and how it is to be said.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Asked<'a> {
    /// The calendar year.
    pub year: i16,
    /// Nominal dollars; today's otherwise.
    pub is_nominal: bool,
    /// The market run the projection is of, as its title names it; none
    /// for the plan's own. A run's tax tables grew at that run's inflation
    /// and are not the plan's.
    pub run: Option<&'a str>,
}

/// What the Ledger says of one year.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Year {
    /// The calendar year.
    pub year: i16,
    /// What its to-dos are titled, with the run it is of: "To do in 2042".
    pub title: String,
    /// The age each person reaches in it: "Jordan turns 67 · Alex turns
    /// 63".
    pub ages: String,
    /// The plan's milestones that fall in it.
    pub milestones: Vec<String>,
    /// What it has the household do, each action a sentence; that there is
    /// nothing, where there is not.
    pub to_do: Vec<String>,
    /// What to watch in it.
    pub warnings: Vec<String>,
    /// What its running totals are titled, with the dollars they are in:
    /// "So far · today's dollars".
    pub so_far_title: String,
    /// What the plan has paid in tax, converted and drawn through it, each
    /// beside its lifetime total, a line only where the plan has any.
    pub so_far: Vec<DetailLine>,
    /// Each account it touches, from its open to its close, and then every
    /// account as one where there are several.
    pub flows: Vec<AccountFlows>,
    /// What it lived on.
    pub money_in: Funds,
    /// Where that went.
    pub money_out: Funds,
    /// What it paid of each kind of tax, and all of it.
    pub tax: Vec<DetailLine>,
    /// What its tax was worked out from, led by the room under the top of
    /// the federal bracket its taxable income reaches; a market run says
    /// no bracket.
    pub worked_from: Vec<DetailLine>,
}

impl Year {
    /// What `projected` says of the year `asked` for; none for a year it
    /// does not reach.
    #[must_use]
    pub fn new(projected: &Projected, tables: &TaxTables, asked: Asked) -> Option<Self> {
        let Projected { plan, projection } = projected;
        let Asked {
            year, is_nominal, ..
        } = asked;
        let row = projection.row(year)?;
        let previous = projection.row(year - 1);
        let dated = milestones(projected, is_nominal).into_iter();
        let mut to_do = actions_said(plan, row, is_nominal);
        if to_do.is_empty() {
            to_do.push(NOTHING_SCHEDULED.to_owned());
        }
        let mut flows = account_flows(plan, previous, row, is_nominal);
        if flows.len() > 1 {
            flows.push(all_accounts(plan, previous, row, is_nominal));
        }
        let room = bracket(plan, tables, row, is_nominal).filter(|_| asked.run.is_none());
        Some(Self {
            year,
            title: title(year, asked.run),
            ages: ages(plan, row),
            so_far_title: format!("{SO_FAR}{JOIN}{}", basis_name(is_nominal)),
            milestones: (dated.filter(|each| each.year == Some(year)))
                .map(|each| each.text)
                .collect(),
            to_do,
            warnings: collect_warnings(plan, tables, row, (!is_nominal).then_some(projection)),
            so_far: so_far(projection, year, is_nominal),
            flows,
            money_in: money_in(plan, row, is_nominal),
            money_out: money_out(plan, row, is_nominal),
            tax: tax_lines(row, is_nominal),
            worked_from: (room.into_iter()).chain(picture(row, is_nominal)).collect(),
        })
    }
}

/// What the year's to-dos are titled, with the `run` they are of.
fn title(year: i16, run: Option<&str>) -> String {
    let to_do = format!("{TO_DO} in {year}");
    run.map_or_else(|| to_do.clone(), |run| format!("{to_do}{JOIN}{run}"))
}

/// Who turns what age in `row`, in household order.
fn ages(plan: &Plan, row: &YearRow) -> String {
    let people = plan.household.people.iter();
    let turning: Vec<String> = people
        .filter_map(|person| {
            let age = row.ages.get(&person.id)?;
            Some(format!("{} turns {age}", person.display_name()))
        })
        .collect();
    turning.join(JOIN)
}

/// Taxes, conversions and withdrawals summed through `year`, each beside
/// what it comes to over the plan, each year's on its own basis as the
/// Overview totals them: a line only where the plan has any.
fn so_far(projection: &Projection, year: i16, is_nominal: bool) -> Vec<DetailLine> {
    let of: [(&str, fn(&YearRow) -> Vec<Dollars>); 3] = [
        ("Taxes", |row| vec![row.taxes.total]),
        ("Converted", |row| vec![row.conversions]),
        ("Withdrawn", |row| {
            row.withdrawals.values().copied().collect()
        }),
    ];
    let mut sums = [(0, 0); 3];
    for row in &projection.years {
        let on_basis = |amount| basis_amount(amount, row.deflator, is_nominal);
        for ((through, lifetime), (_, amounts)) in sums.iter_mut().zip(of) {
            let paid: Dollars = amounts(row).into_iter().map(on_basis).sum();
            *lifetime += paid;
            if row.year <= year {
                *through += paid;
            }
        }
    }
    (sums.into_iter().zip(of))
        .filter(|&((_, lifetime), _)| lifetime != 0)
        .map(|((through, lifetime), (what, _))| DetailLine {
            label: what.to_owned(),
            amount: format!("{} of {}", compact_money(through), compact_money(lifetime)),
        })
        .collect()
}
