//! The deterministic year-by-year projection.
//!
//! Amounts are computed in nominal dollars; every row carries the year's
//! cumulative inflation factor (`deflator`), so today's-dollar views divide
//! by it. The engine assumes the plan has passed [`crate::plan::Plan::validate`];
//! on an invalid plan it stays panic-free but its numbers are unspecified.

mod benefit;
mod collect;
mod contribute;
mod flows;
mod invest;
mod path;
mod residence;
mod resolve;
mod settle;
mod year;

pub(crate) use benefit::benefit_params;
pub use path::MarketPath;
use residence::check_modeled_states;
pub use resolve::{Timeline, Window};

use std::collections::BTreeMap;

use serde::Serialize;

pub(crate) use crate::params::scale;
use crate::params::{Inflation, TaxTables};
use crate::plan::{Dollars, Issue, Plan, TreatmentClass};

/// One projected plan: a row per calendar year.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Projection {
    /// Rows from the plan's start year through the horizon.
    pub years: Vec<YearRow>,
}

/// Taxes assessed for one year.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Taxes {
    /// Tax on ordinary income.
    pub ordinary: Dollars,
    /// Tax on long-term capital gains.
    pub ltcg: Dollars,
    /// Early-withdrawal penalties.
    pub penalty: Dollars,
    /// Income tax of the state lived in that year.
    pub state: Dollars,
    /// The taxable portion of Social Security benefits (informational).
    pub taxable_social_security: Dollars,
    /// Ordinary taxable income after the standard deduction, floored at
    /// zero (informational; what the bracket walk was fed).
    pub ordinary_taxable: Dollars,
    /// MAGI proxy - ordinary income plus gains plus taxable Social
    /// Security (informational; drives IRMAA lookback and cliffs).
    pub magi: Dollars,
    /// Ordinary, gains and state tax plus penalties.
    pub total: Dollars,
    /// What of the year's traditional IRA contributions the MAGI let be
    /// deducted; settled with the taxes, not a figure of them.
    #[serde(skip)]
    pub(crate) ira_deducted: Dollars,
}

/// End-of-year balances aggregated by treatment class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct ClassTotals {
    /// Taxable: brokerage and cash.
    pub taxable: Dollars,
    /// Tax-deferred.
    pub deferred: Dollars,
    /// Roth.
    pub roth: Dollars,
    /// HSA.
    pub hsa: Dollars,
}

impl ClassTotals {
    /// The total for one treatment class.
    #[must_use]
    pub fn get(&self, class: TreatmentClass) -> Dollars {
        match class {
            TreatmentClass::Taxable => self.taxable,
            TreatmentClass::Deferred => self.deferred,
            TreatmentClass::Roth => self.roth,
            TreatmentClass::Hsa => self.hsa,
        }
    }

    pub(crate) fn add(&mut self, class: TreatmentClass, amount: Dollars) {
        match class {
            TreatmentClass::Taxable => self.taxable += amount,
            TreatmentClass::Deferred => self.deferred += amount,
            TreatmentClass::Roth => self.roth += amount,
            TreatmentClass::Hsa => self.hsa += amount,
        }
    }
}

/// How a year's contribution into an account came to be what it is.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContributionNote {
    /// An item paid a share of an income's gross, at this year's rate.
    Share {
        /// The share paid this year, the step applied.
        rate: f64,
        /// The income id.
        of: String,
    },
    /// An item paid the year's employee limit.
    Maximum,
    /// An employer matched the employee's contributions to the account.
    Match {
        /// The share of the employee's contribution matched.
        rate: f64,
        /// The share of the income's gross matched up to.
        up_to: f64,
        /// The income id.
        of: String,
    },
    /// Part of the employee amount is after-tax money, kept as basis.
    AfterTax {
        /// Nominal dollars paid after tax.
        amount: Dollars,
    },
    /// An employee amount was held to the owner's limit for the year.
    HeldToLimit,
    /// The employer amount was cut to keep the account under the overall
    /// cap on what a plan takes in a year.
    HeldToOverall,
    /// The year's MAGI is inside or above the band over which a Roth IRA
    /// contribution is not allowed.
    RothIraPhaseOut,
    /// Part of a traditional IRA contribution was not deductible, the owner
    /// being covered by a workplace plan with MAGI in the band, and is kept
    /// as basis.
    NotDeducted {
        /// Nominal dollars not deducted.
        amount: Dollars,
    },
}

/// One executed instruction in a projected year, with post-clamp nominal
/// amounts; what a user following the plan would actually do.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// A scheduled transfer that fired (lock-deferred ones in their
    /// actual firing year).
    Transfer {
        /// Source account id.
        from: String,
        /// Destination account id.
        to: String,
        /// Nominal dollars moved.
        amount: Dollars,
    },
    /// One account's required minimum distribution.
    Rmd {
        /// The distributing account id.
        account: String,
        /// Nominal dollars distributed.
        amount: Dollars,
    },
    /// One account's contributions for the year.
    Contribution {
        /// The receiving account id.
        account: String,
        /// Employee contribution from cash flow.
        employee: Dollars,
        /// Employer contribution landing directly.
        employer: Dollars,
        /// How the amounts arose, where a figure alone does not say.
        notes: Vec<ContributionNote>,
    },
    /// One executed Roth conversion step.
    Conversion {
        /// Source account id.
        from: String,
        /// Destination account id.
        to: String,
        /// Nominal dollars converted.
        amount: Dollars,
    },
    /// A withdrawal beyond RMDs that funded spending and taxes.
    Withdrawal {
        /// The account drained.
        account: String,
        /// Nominal dollars withdrawn.
        amount: Dollars,
    },
    /// The year's unspent income, swept into the surplus account.
    Surplus {
        /// The account swept into.
        account: String,
        /// Nominal dollars swept.
        amount: Dollars,
    },
}

/// One projected calendar year, in nominal dollars.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct YearRow {
    /// The calendar year.
    pub year: i16,
    /// Age each person reaches during the year, by person id.
    pub ages: BTreeMap<String, u8>,
    /// Cumulative inflation factor since plan start; nominal ÷ this = today's
    /// dollars.
    pub deflator: f64,
    /// Gross income by source label, Social Security included.
    pub income: BTreeMap<String, Dollars>,
    /// Sum of the income map.
    pub total_income: Dollars,
    /// Spending for the year.
    pub expenses: Dollars,
    /// Medicare surcharges and crossed-cliff costs spent this year.
    pub medicare: Dollars,
    /// Employee contributions paid from cash flow.
    pub contributions_employee: Dollars,
    /// Employer contributions landing in accounts.
    pub contributions_employer: Dollars,
    /// Required minimum distributions taken.
    pub rmds: Dollars,
    /// Roth conversions executed.
    pub conversions: Dollars,
    /// Money withdrawn per account id (RMDs included).
    pub withdrawals: BTreeMap<String, Dollars>,
    /// The year's growth per account id, on what the account opened with;
    /// an account that grew nothing is left out.
    pub growth: BTreeMap<String, Dollars>,
    /// What was executed, in execution order; funding withdrawals, then the
    /// surplus swept, last.
    pub actions: Vec<Action>,
    /// The year's taxes.
    pub taxes: Taxes,
    /// Unspent income swept to the surplus account.
    pub surplus: Dollars,
    /// Spending the accounts could not cover.
    pub unfunded: Dollars,
    /// End-of-year balance per account id.
    pub balances: BTreeMap<String, Dollars>,
    /// End-of-year balances by treatment class.
    pub class_totals: ClassTotals,
    /// Sum of all end-of-year balances.
    pub net_worth: Dollars,
}

impl YearRow {
    /// Total withdrawn across all accounts, RMDs included.
    #[must_use]
    pub fn total_withdrawals(&self) -> Dollars {
        self.withdrawals.values().sum()
    }
}

/// Headline figures aggregated over a whole projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Summary {
    /// Net worth at the horizon.
    pub final_net_worth: Dollars,
    /// Highest year-end net worth reached.
    pub peak_net_worth: Dollars,
    /// The year the peak occurs; the first such year on ties.
    pub peak_year: i16,
    /// Total taxes assessed across the projection.
    pub lifetime_taxes: Dollars,
    /// Total Roth conversions executed.
    pub lifetime_conversions: Dollars,
    /// Total spending the accounts could not cover.
    pub lifetime_unfunded: Dollars,
    /// Total Medicare surcharges and cliff costs.
    pub lifetime_medicare: Dollars,
    /// First year with an uncovered shortfall; `None` when fully funded.
    pub first_unfunded_year: Option<i16>,
    /// Deferred-class balance at the horizon.
    pub final_deferred: Dollars,
}

impl Projection {
    /// Aggregates the projection into headline figures. With `deflated`,
    /// each year's amounts convert to today's dollars before summing and
    /// comparing; the peak is picked on the displayed basis. An empty
    /// projection yields zeros.
    #[must_use]
    pub fn summary(&self, deflated: bool) -> Summary {
        let money = |amount: Dollars, deflator: f64| {
            if deflated {
                deflate(amount, deflator)
            } else {
                amount
            }
        };
        let mut peak: Option<(Dollars, i16)> = None;
        let mut taxes = 0;
        let mut conversions = 0;
        let mut unfunded = 0;
        let mut medicare = 0;
        let mut first_unfunded_year = None;
        for row in &self.years {
            let net_worth = money(row.net_worth, row.deflator);
            if peak.is_none_or(|(value, _)| net_worth > value) {
                peak = Some((net_worth, row.year));
            }
            taxes += money(row.taxes.total, row.deflator);
            conversions += money(row.conversions, row.deflator);
            unfunded += money(row.unfunded, row.deflator);
            medicare += money(row.medicare, row.deflator);
            if row.unfunded > 0 && first_unfunded_year.is_none() {
                first_unfunded_year = Some(row.year);
            }
        }
        let (peak_net_worth, peak_year) = peak.unwrap_or((0, 0));
        let last = self.years.last();
        Summary {
            final_net_worth: last.map_or(0, |row| money(row.net_worth, row.deflator)),
            peak_net_worth,
            peak_year,
            lifetime_taxes: taxes,
            lifetime_conversions: conversions,
            lifetime_unfunded: unfunded,
            lifetime_medicare: medicare,
            first_unfunded_year,
            final_deferred: last.map_or(0, |row| money(row.class_totals.deferred, row.deflator)),
        }
    }
}

/// The lowest nominal MAGI threshold among cliffs active in `year`, for the
/// optimizer's automatic ceiling.
pub(crate) fn min_active_cliff_threshold(plan: &Plan, year: i16) -> Option<Dollars> {
    if plan.cliffs.is_empty() {
        return None;
    }
    let resolver = resolve::Resolver::new(plan);
    let default_end = collect::default_cliff_end(plan);
    plan.cliffs
        .iter()
        .filter(|cliff| collect::is_cliff_active(plan, &resolver, default_end, cliff, year))
        .map(|cliff| {
            let years = i32::from(year - plan.plan.start_year);
            scale(
                cliff.magi_over,
                cliff.cola.factor(inflation_factor(plan, year), years),
            )
        })
        .min()
}

/// The total IRMAA surcharge that `magi` in `year` buys when its premiums
/// land [`crate::tax::IRMAA_LOOKBACK_YEARS`] later: zero without an opted-in
/// `[medicare]` section, past the plan's horizon, or with no one covered
/// then.
#[must_use]
pub fn irmaa_purchase(
    plan: &Plan,
    tables: &crate::params::TaxTables,
    year: i16,
    magi: Dollars,
) -> Dollars {
    let Some(medicare) = &plan.medicare else {
        return 0;
    };
    let premium_year = year + crate::tax::IRMAA_LOOKBACK_YEARS;
    if premium_year > horizon_year(plan) {
        return 0;
    }
    let covered = covered_count(plan, premium_year);
    if covered == 0 {
        return 0;
    }
    let params = tables.params_for(premium_year, &plan_inflation(plan));
    let per_person =
        crate::tax::irmaa_surcharge(&params, plan.household.filing, magi, medicare.part_d);
    per_person * covered as Dollars
}

/// Converts a nominal amount to today's dollars using its year's deflator.
#[must_use]
pub fn deflate(amount: Dollars, deflator: f64) -> Dollars {
    (amount as f64 / deflator).round() as Dollars
}

impl Projection {
    /// `amount` deflated by `year`'s deflator; a year outside the
    /// projection returns it unchanged.
    #[must_use]
    pub fn deflate_in(&self, year: i16, amount: Dollars) -> Dollars {
        self.years
            .iter()
            .find(|row| row.year == year)
            .map_or(amount, |row| deflate(amount, row.deflator))
    }
}

/// The full validation contract the engine assumes: structural issues from
/// [`Plan::validate`] first, and only on a structurally clean plan the
/// checks that read the tax tables - that each state lived in is modeled,
/// and that a computed Social Security benefit has its claim age and its
/// formula amounts. Every surface (CLI, MCP, TUI) runs this before
/// [`project`].
#[must_use]
pub fn validate_plan(plan: &Plan, tables: &TaxTables) -> Vec<Issue> {
    let issues = plan.validate();
    if !issues.is_empty() {
        return issues;
    }
    let mut issues = check_modeled_states(plan, tables);
    issues.extend(benefit::check_derived_claims(plan, tables));
    issues
}

/// Projects a valid plan year by year against the tax tables, in the
/// market the plan states.
#[must_use]
pub fn project(plan: &Plan, tables: &TaxTables) -> Projection {
    project_on(plan, tables, &MarketPath::expected(plan))
}

/// Projects a valid plan year by year against the tax tables, in `path`.
#[must_use]
pub fn project_on(plan: &Plan, tables: &TaxTables, path: &MarketPath) -> Projection {
    year::Simulation::new(plan, tables, path).run()
}

/// How many of the household are covered by Medicare in `year`.
pub(crate) fn covered_count(plan: &Plan, year: i16) -> usize {
    plan.household
        .people
        .iter()
        .filter(|person| crate::tax::is_medicare_covered(person, year))
        .count()
}

pub(crate) fn horizon_year(plan: &Plan) -> i16 {
    latest_birthday_year(plan, plan.plan.horizon_age)
}

/// The last calendar year in which any household member turns `age`.
pub(crate) fn latest_birthday_year(plan: &Plan, age: u8) -> i16 {
    plan.household
        .people
        .iter()
        .map(|person| person.birth.year() + i16::from(age))
        .max()
        .unwrap_or(plan.plan.start_year)
}

/// How prices move at the plan's own inflation, every year.
pub(crate) fn plan_inflation(plan: &Plan) -> Inflation {
    Inflation::constant(plan.plan.inflation)
}

/// What a start-year price costs in `year` at the plan's own inflation.
pub(crate) fn inflation_factor(plan: &Plan, year: i16) -> f64 {
    plan_inflation(plan).factor(plan.plan.start_year, year)
}

/// How many years a plan projects.
pub(crate) fn horizon_len(plan: &Plan) -> usize {
    usize::try_from(horizon_year(plan) - plan.plan.start_year + 1).unwrap_or_default()
}
