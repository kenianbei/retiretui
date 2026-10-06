//! One year in full: who turns what age, the milestones that fall in it,
//! what to do and what to watch, how far the plan has come, each account's
//! flows, where its money came from and went, and its tax.

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Item, Plan};
use retiretui_engine::project::{Projection, YearRow};
use serde::Serialize;

use super::flows::{AccountFlows, account_flows, all_accounts};
use super::funds::{DetailLine, Funds, money_in, money_out};
use super::tax::{bracket, picture, tax_lines};
use crate::actions::{NOTHING_SCHEDULED, actions_said, collect_warnings};
use crate::overview::milestones;
use crate::present::compact_money;
use crate::session::Projected;
use crate::table::basis_amount;

const JOIN: &str = " · ";

/// Which year is asked for, and how it is to be said.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Asked {
    /// The calendar year.
    pub year: i16,
    /// Nominal dollars; today's otherwise.
    pub is_nominal: bool,
    /// Whether the projection is a market run's, whose tax tables grew at
    /// that run's inflation and are not the plan's own.
    pub is_run: bool,
}

/// What the Ledger says of one year.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Year {
    /// The calendar year.
    pub year: i16,
    /// The age each person reaches in it: "Jordan turns 67 · Alex turns 63".
    pub ages: String,
    /// The plan's milestones that fall in it.
    pub milestones: Vec<String>,
    /// What it has the household do, each action a sentence; that there is
    /// nothing, where there is not.
    pub to_do: Vec<String>,
    /// What to watch in it.
    pub warnings: Vec<String>,
    /// What the plan has paid, converted and drawn through it, each beside
    /// its lifetime total; none where every total is nothing.
    pub so_far: Option<String>,
    /// Each account it touches, from its open to its close.
    pub flows: Vec<AccountFlows>,
    /// Every account as one, where there are several.
    pub all_accounts: Option<AccountFlows>,
    /// What it lived on.
    pub money_in: Funds,
    /// Where that went.
    pub money_out: Funds,
    /// What it paid of each kind of tax, and all of it.
    pub tax: Vec<DetailLine>,
    /// The federal bracket its taxable income reaches and the room under
    /// that bracket's top; none for a market run.
    pub bracket: Option<DetailLine>,
    /// What its tax was worked out from.
    pub picture: Vec<DetailLine>,
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
        let flows = account_flows(plan, previous, row, is_nominal);
        let together = || all_accounts(plan, previous, row, is_nominal);
        Some(Self {
            year,
            ages: ages(plan, row),
            milestones: (dated.filter(|each| each.year == Some(year)))
                .map(|each| each.text)
                .collect(),
            to_do,
            warnings: collect_warnings(plan, tables, row, (!is_nominal).then_some(projection)),
            so_far: so_far(projection, year, is_nominal),
            all_accounts: (flows.len() > 1).then(together),
            flows,
            money_in: money_in(plan, row, is_nominal),
            money_out: money_out(row, is_nominal),
            tax: tax_lines(row, is_nominal),
            bracket: (!asked.is_run)
                .then(|| bracket(plan, tables, row, is_nominal))
                .flatten(),
            picture: picture(row, is_nominal),
        })
    }
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

/// Taxes, conversions and withdrawals summed through `year` beside what
/// each comes to over the plan, each year's on its own basis as the
/// Overview totals them: a clause only where the plan has any.
fn so_far(projection: &Projection, year: i16, is_nominal: bool) -> Option<String> {
    let of: [(&str, fn(&YearRow) -> Vec<Dollars>); 3] = [
        ("taxes", |row| vec![row.taxes.total]),
        ("converted", |row| vec![row.conversions]),
        ("withdrawn", |row| {
            row.withdrawals.values().copied().collect()
        }),
    ];
    let clauses: Vec<String> = of
        .into_iter()
        .filter_map(|(what, amounts)| {
            let summed = |through: i16| -> Dollars {
                let years = projection.years.iter().filter(|row| row.year <= through);
                years
                    .flat_map(|row| {
                        let on_basis = |amount| basis_amount(amount, row.deflator, is_nominal);
                        amounts(row).into_iter().map(on_basis)
                    })
                    .sum()
            };
            let lifetime = summed(i16::MAX);
            let so_far = compact_money(summed(year));
            (lifetime != 0).then(|| format!("{so_far} of {} {what}", compact_money(lifetime)))
        })
        .collect();
    (!clauses.is_empty()).then(|| format!("So far: {}", clauses.join(JOIN)))
}
