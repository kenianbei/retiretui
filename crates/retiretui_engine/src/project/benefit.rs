//! What a `social-security` income without an `amount` pays: the owner's
//! earnings record, extended with the salary the walk has paid them, through
//! the benefit formula the first year the income is active.

use crate::params::{BenefitParams, TaxTables};
use std::collections::BTreeMap;

use crate::plan::{ColaSpec, Dollars, Income, IncomeKind, Issue, Person, Plan, push_issue};
use crate::tax;

use super::resolve::trigger_month;
use super::year::Simulation;
use super::{plan_inflation, scale};

impl Simulation<'_> {
    /// Records a year's salary as covered earnings of its owner, nominal
    /// as SSA keeps a record.
    pub(super) fn record_covered(&mut self, owner: &str, year: i16, nominal: Dollars) {
        let Some(person) = self.plan.person(owner) else {
            return;
        };
        *self
            .covered
            .entry(person.id.as_str())
            .or_default()
            .entry(year)
            .or_default() += nominal;
    }

    /// What a Social Security income pays of `nominal` in `year`: in the
    /// year it is first paid for, the months from that one; all of it in
    /// any later year, and for any other income.
    pub(super) fn social_security_paid(
        &self,
        index: usize,
        year: i16,
        nominal: Dollars,
    ) -> Dollars {
        match self.first_paid[index] {
            Some(first_paid) => scale(nominal, tax::claim_year_share(first_paid, year)),
            None => nominal,
        }
    }

    /// The benefit of the income at `index` in `year`, computed at its first
    /// active year - the claim - and kept.
    pub(super) fn derived_benefit(&mut self, index: usize, year: i16) -> Dollars {
        let derived = self.benefits[index].unwrap_or_else(|| self.compute_benefit(index, year));
        self.benefits[index] = Some(derived);
        if year <= derived.claim_year {
            derived.claim_year_amount
        } else {
            derived.later_amount
        }
    }

    /// The benefit priced at its claim, in start-year dollars: carried from
    /// the age-62 year by the COLAs to the start, so that the income's own
    /// escalation reaches the claim as the COLAs to come. A claim past full
    /// retirement age is paid its claim year's credits from the next
    /// January, as SSA pays them, unless it is made at 70 or later.
    fn compute_benefit(&self, index: usize, first_active_year: i16) -> Derived {
        let income = &self.plan.income[index];
        let owner = self.plan.person(&income.owner);
        let (Some(owner), Some(params)) = (owner, benefit_params(self.plan, self.tables)) else {
            return Derived::default();
        };
        let earnings = self.claim_record(owner, &params, first_active_year);
        let claim = self.first_paid[index]
            .unwrap_or_else(|| tax::attained_month(owner.birth, owner.age_in(first_active_year)));
        let claim_year = year_of(claim);
        let age = tax::age_months(owner.birth, claim);
        let january = tax::age_months(owner.birth, tax::month_index(claim_year, 1));
        let claim_year_age = tax::claim_year_age(owner.birth.year(), age, january);
        let eligibility_year = owner.eligibility_year();
        let colas = self.colas_to_start(&params, income.cola, eligibility_year);
        let to_start = 1.0 / self.cola_factor(income.cola, eligibility_year.max(self.start_year));
        let in_start_dollars = |age| {
            let benefit =
                tax::social_security_benefit(&params, owner.birth.year(), age, &earnings, &colas);
            scale(benefit, to_start)
        };
        Derived {
            claim_year,
            claim_year_amount: in_start_dollars(claim_year_age),
            later_amount: in_start_dollars(age),
        }
    }

    /// `owner`'s record for a claim first paid in `first_active_year`: their
    /// own - where they have none, a career before the plan at the salary
    /// its first year pays them - with each year it lacks before the claim
    /// filled from the salary paid so far.
    fn claim_record(
        &self,
        owner: &Person,
        params: &BenefitParams,
        first_active_year: i16,
    ) -> BTreeMap<i16, Dollars> {
        let paid = self.covered.get(owner.id.as_str());
        let first_salary = paid.and_then(|years| years.get(&self.start_year)).copied();
        let mut earnings = match first_salary {
            Some(salary) if owner.earnings.is_empty() => {
                tax::career_before(params, owner.birth.year(), salary, self.start_year)
            }
            _ => owner.earnings.clone(),
        };
        let covered = paid.into_iter().flatten();
        for (&year, &amount) in covered.filter(|&(&year, _)| year < first_active_year) {
            earnings.entry(year).or_insert(amount);
        }
        earnings
    }

    /// The COLA of each year from `eligibility_year` to the plan's start:
    /// SSA's where it has published one, the income's own rate where not.
    fn colas_to_start(
        &self,
        params: &BenefitParams,
        cola: ColaSpec,
        eligibility_year: i16,
    ) -> Vec<f64> {
        let assumed = |year| self.cola_factor(cola, year + 1) / self.cola_factor(cola, year) - 1.0;
        (eligibility_year..self.start_year)
            .map(|year| {
                params
                    .cola
                    .get(&year)
                    .copied()
                    .unwrap_or_else(|| assumed(year))
            })
            .collect()
    }
}

/// A computed benefit, kept once the claim has priced it: what its claim
/// year pays, and every year after.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Derived {
    claim_year: i16,
    claim_year_amount: Dollars,
    later_amount: Dollars,
}

/// The first month each of the plan's incomes is paid for, where it is a
/// Social Security benefit with a stated start: the month the start fires,
/// or, when that is the month 62 is attained, the first month it is held
/// throughout.
pub(super) fn first_paid_months(plan: &Plan) -> Vec<Option<i32>> {
    let first_paid = |income: &Income| {
        if income.kind != IncomeKind::SocialSecurity {
            return None;
        }
        let owner = plan.person(&income.owner)?;
        let fired = trigger_month(plan, income.start.as_ref()?)?;
        Some(
            if tax::age_months(owner.birth, fired) == tax::EARLIEST_CLAIM_MONTHS {
                tax::first_claim_month(owner.birth)
            } else {
                fired
            },
        )
    };
    plan.income.iter().map(first_paid).collect()
}

fn year_of(month: i32) -> i16 {
    month.div_euclid(tax::MONTHS_PER_YEAR) as i16
}

/// The benefit formula's parameters for `plan`: the start year's table,
/// its wage growth replaced by the plan's where the plan states one; none
/// where that table carries no formula.
#[must_use]
pub fn benefit_params(plan: &Plan, tables: &TaxTables) -> Option<BenefitParams> {
    let mut params = tables
        .params_for(plan.plan.start_year, &plan_inflation(plan))
        .social_security
        .benefit?;
    if let Some(rate) = plan.plan.wage_growth {
        params.wage_growth = rate;
    }
    Some(params)
}

/// What a computed benefit needs beyond the structural rules: a claim at 62
/// or later, and a start-year table that carries the benefit formula's
/// amounts.
pub(super) fn check_derived_claims(plan: &Plan, tables: &TaxTables) -> Vec<Issue> {
    let mut issues = Vec::new();
    if !plan.income.iter().any(Income::is_derived) {
        return issues;
    }
    let has_params = tables
        .params_for(plan.plan.start_year, &plan_inflation(plan))
        .social_security
        .benefit
        .is_some();
    for (i, income) in plan
        .income
        .iter()
        .enumerate()
        .filter(|(_, income)| income.is_derived())
    {
        let claim = (income.start.as_ref()).and_then(|start| trigger_month(plan, start));
        let Some((owner, claim)) = plan.person(&income.owner).zip(claim) else {
            continue;
        };
        let age = tax::age_months(owner.birth, claim);
        if age < tax::EARLIEST_CLAIM_MONTHS {
            push_issue(
                &mut issues,
                format!("income[{i}].start"),
                format!(
                    "a computed benefit needs a claim at {} or later; this claims at {}",
                    tax::EARLIEST_CLAIM_AGE,
                    age_in_words(age)
                ),
            );
        }
        if !has_params {
            push_issue(
                &mut issues,
                format!("income[{i}].amount"),
                format!(
                    "the tax parameters for {} carry no [social-security.benefit] table to compute it from",
                    plan.plan.start_year
                ),
            );
        }
    }
    issues
}

/// An age in months as years, and months where there are any.
fn age_in_words(age: i32) -> String {
    let (years, months) = (
        age.div_euclid(tax::MONTHS_PER_YEAR),
        age.rem_euclid(tax::MONTHS_PER_YEAR),
    );
    match months {
        0 => years.to_string(),
        1 => format!("{years} and 1 month"),
        _ => format!("{years} and {months} months"),
    }
}
