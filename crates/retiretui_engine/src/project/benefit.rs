//! What a `social-security` income without an `amount` pays: the owner's
//! earnings record, extended with the salary the walk has paid them, through
//! the benefit formula the first year the income is active.

use crate::params::{BenefitParams, TaxTables};
use crate::plan::{Dollars, Income, IncomeKind, Issue, Plan, TriggerForm, push_issue};
use crate::tax;

use super::resolve::Resolver;
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
    /// year a start on its owner's age fires, the months from the one the
    /// age is attained in; all of it in any other year, and for any other
    /// income.
    pub(super) fn social_security_paid(
        &self,
        income: &Income,
        year: i16,
        nominal: Dollars,
    ) -> Dollars {
        if income.kind != IncomeKind::SocialSecurity {
            return nominal;
        }
        let start = income.start.as_ref().and_then(|start| start.form().ok());
        let Some(TriggerForm::Age { owner, years }) = start else {
            return nominal;
        };
        if owner != income.owner {
            return nominal;
        }
        let claim_age = i16::from(years);
        match self.plan.person(owner) {
            Some(person) if person.age_in_year(year) == claim_age => {
                scale(nominal, tax::claim_year_share(person.birth, claim_age))
            }
            _ => nominal,
        }
    }

    /// The benefit of the income at `index`, computed at its first active
    /// year - the claim - and kept.
    pub(super) fn derived_benefit(&mut self, index: usize, claim_year: i16) -> Dollars {
        if let Some(benefit) = self.benefits[index] {
            return benefit;
        }
        let benefit = self.compute_benefit(index, claim_year);
        self.benefits[index] = Some(benefit);
        benefit
    }

    /// The owner's record, each year it lacks before the claim filled from
    /// the salary paid so far, capped at that year's wage base; the benefit
    /// in start-year dollars, so that the income's own escalation reaches
    /// the claim as the COLAs SSA adds from the age-62 year.
    fn compute_benefit(&self, index: usize, claim_year: i16) -> Dollars {
        let income = &self.plan.income[index];
        let owner = self.plan.person(&income.owner);
        let (Some(owner), Some(params)) = (owner, benefit_params(self.plan, self.tables)) else {
            return 0;
        };
        let mut earnings = owner.earnings.clone();
        let covered = self.covered.get(owner.id.as_str()).into_iter().flatten();
        for (&year, &amount) in covered.filter(|&(&year, _)| year < claim_year) {
            earnings.entry(year).or_insert(amount);
        }
        let at_eligibility = tax::social_security_benefit(
            &params,
            owner.birth.year(),
            owner.age_in_year(claim_year),
            &earnings,
        );
        let eligibility_year = owner.birth.year() + tax::EARLIEST_CLAIM_AGE;
        scale(
            at_eligibility,
            1.0 / self.cola_factor(income.cola, eligibility_year),
        )
    }
}

/// The benefit formula's parameters for `plan`: the start year's table,
/// its wage growth replaced by the plan's where the plan states one.
pub(crate) fn benefit_params(plan: &Plan, tables: &TaxTables) -> Option<BenefitParams> {
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
    let resolver = Resolver::new(plan);
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
        let claim_year = income
            .start
            .as_ref()
            .and_then(|start| resolver.trigger_year(plan, start));
        let Some((owner, claim_year)) = plan.person(&income.owner).zip(claim_year) else {
            continue;
        };
        let age = owner.age_in_year(claim_year);
        if age < tax::EARLIEST_CLAIM_AGE {
            push_issue(
                &mut issues,
                format!("income[{i}].start"),
                format!(
                    "a computed benefit needs a claim at {} or later; this claims at {age}",
                    tax::EARLIEST_CLAIM_AGE
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
