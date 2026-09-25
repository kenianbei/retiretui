//! What a person's Social Security benefit comes to before anyone searches
//! it: the benefit at the ages that frame the choice, and a career at their
//! salary where no statement has been imported.

use std::collections::BTreeMap;

use super::claims::{Claim, computed_income};
use crate::params::TaxTables;
use crate::plan::{Dollars, IncomeKind, Person, Plan};
use crate::project::benefit_params;
use crate::project::{deflate, horizon_year, project};
use crate::tax::{self, EARLIEST_CLAIM_AGE, FIRST_WORKING_AGE, LATEST_CREDIT_AGE, MONTHS_PER_YEAR};

/// `person`'s monthly benefit in today's dollars claimed at 62, at the first
/// whole age at or past full retirement age, and at 70: each read off the
/// first whole year paid in a projection of the plan with their
/// `social-security` income computed - made up where they have none - and
/// claimed at that age. `None` for an age the plan no longer reaches or
/// whose first whole year is past the horizon, and at every age for a
/// person with no earnings record. `plan` must be valid.
#[must_use]
pub fn benefit_estimates(plan: &Plan, tables: &TaxTables, person: &str) -> [Option<Dollars>; 3] {
    let Some(owner) = plan
        .person(person)
        .filter(|owner| !owner.earnings.is_empty())
    else {
        return [None; 3];
    };
    let months = tax::full_retirement_months(owner.birth.year());
    let full = (months + MONTHS_PER_YEAR - 1) / MONTHS_PER_YEAR;
    let mut estimating = plan.clone();
    let held = estimating
        .income
        .iter()
        .position(|income| income.is_benefit_of(&owner.id));
    let index = held.unwrap_or_else(|| {
        estimating.income.push(computed_income(&owner.id));
        estimating.income.len() - 1
    });
    estimating.income[index].amount = None;
    [EARLIEST_CLAIM_AGE, full as i16, LATEST_CREDIT_AGE]
        .map(|age| estimate_at(&mut estimating, tables, index, owner, age))
}

/// One age's estimate: `estimating` is the plan with the person's income
/// at `index` computed, whose `start` each age overwrites.
fn estimate_at(
    estimating: &mut Plan,
    tables: &TaxTables,
    index: usize,
    owner: &Person,
    age: i16,
) -> Option<Dollars> {
    let first_whole_year = owner.birth.year() + age + 1;
    if age < owner.age_in_year(estimating.plan.start_year)
        || first_whole_year > horizon_year(estimating)
    {
        return None;
    }
    let income = &mut estimating.income[index];
    let claim = Claim {
        income: income.id.clone(),
        owner: owner.id.clone(),
        age: u8::try_from(age).ok()?,
    };
    income.start = Some(claim.trigger());
    let projection = project(estimating, tables);
    let row = projection
        .years
        .iter()
        .find(|row| row.year == first_whole_year)?;
    let paid = *row.income.get(&claim.income)?;
    Some(deflate(paid, row.deflator * f64::from(MONTHS_PER_YEAR)))
}

/// A career for `person` at the salary the plan pays them in its start
/// year, today's dollars, through [`tax::earnings_at_wage`] from
/// [`FIRST_WORKING_AGE`] to the year before the plan. `plan` must be valid.
///
/// # Errors
///
/// Says why when `person` is unknown, the start year's table carries no
/// benefit formula, the plan pays them no salary in its start year, or no
/// working year falls before it.
pub fn career_at_salary(
    plan: &Plan,
    tables: &TaxTables,
    person: &str,
) -> Result<BTreeMap<i16, Dollars>, String> {
    let owner = plan
        .person(person)
        .ok_or_else(|| format!("no person `{person}`"))?;
    let start_year = plan.plan.start_year;
    let params = benefit_params(plan, tables)
        .ok_or_else(|| format!("the {start_year} table carries no benefit formula"))?;
    let salary = salary_in_start_year(plan, tables, person);
    if salary <= 0 {
        return Err(format!(
            "{person} is paid no salary in {start_year} to fill a career from; import a statement"
        ));
    }
    let from = owner.birth.year() + FIRST_WORKING_AGE;
    let career = tax::earnings_at_wage(&params, salary, start_year, from..=start_year - 1);
    if career.is_empty() {
        return Err(format!("{person} has no working year before {start_year}"));
    }
    Ok(career)
}

/// What the plan's first year pays `person` in salary, today's dollars.
fn salary_in_start_year(plan: &Plan, tables: &TaxTables, person: &str) -> Dollars {
    let projection = project(plan, tables);
    let Some(first) = projection.years.first() else {
        return 0;
    };
    let nominal: Dollars = plan
        .income
        .iter()
        .filter(|income| income.kind == IncomeKind::Salary && income.owner == person)
        .filter_map(|income| first.income.get(&income.id))
        .sum();
    deflate(nominal, first.deflator)
}
