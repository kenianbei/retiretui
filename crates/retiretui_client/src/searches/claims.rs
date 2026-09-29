//! The Social Security claim search as every surface says it: a set of
//! claims in words, each person's row of the People table, what can be done
//! for them, and those of it that edit the plan.

use retiretui_engine::optimize::{Claim, ClaimSearch, career_at_salary};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Income, Item, Person, Plan};
use retiretui_engine::tax::MONTHS_PER_YEAR;
use serde::{Deserialize, Serialize};

use crate::present::compact_money;

/// What the options say before anything is searched.
pub const NOTHING_SEARCHED: &str = "Every age each computed Social Security benefit can be claimed at is ranked here, jointly for the household, as soon as the plan is valid.";

/// What a claim the plan does not pay says.
const NO_CLAIM: &str = "none";

/// What a held claim's income says.
pub const HELD: &str = "held";

/// The People table's columns.
pub const PEOPLE_COLUMNS: [&str; 6] = ["Person", "Record", "Income", "62", "FRA", "70"];

/// What an action on nobody is refused with.
pub const NOBODY: &str = "no one in the household";

/// What an estimate that cannot be made says.
const NO_ESTIMATE: &str = "-";

/// Claims as a sentence says them: `Ann at 70, Bob at 67`.
#[must_use]
pub fn said(plan: &Plan, claims: &[Claim]) -> String {
    let each: Vec<String> = claims
        .iter()
        .map(|claim| format!("{} at {}", plan.person_name(&claim.owner), claim.age))
        .collect();
    each.join(", ")
}

/// What is asked before `claims` are taken into `plan`.
#[must_use]
pub fn take_question(plan: &Plan, claims: &[Claim]) -> String {
    format!("Take these claims? {}.", said(plan, claims))
}

/// What is said once `claims` are taken into `plan`.
#[must_use]
pub fn taken(plan: &Plan, claims: &[Claim]) -> String {
    format!("claimed {}", said(plan, claims))
}

/// The name of each person the search claims for, in its claims' order.
#[must_use]
pub fn claimants(plan: &Plan, search: &ClaimSearch) -> Vec<String> {
    let best = search.candidates.first();
    best.map(|best| {
        (best.claims.iter())
            .map(|claim| plan.person_name(&claim.owner).to_owned())
            .collect()
    })
    .unwrap_or_default()
}

/// The options' columns: each person claimed for, what an option ends
/// with against the plan, then the figures.
#[must_use]
pub fn option_columns(plan: &Plan, search: &ClaimSearch) -> Vec<String> {
    let figures = std::iter::once(super::AGAINST_PLAN).chain(super::FIGURES);
    let mut columns = claimants(plan, search);
    columns.extend(figures.map(str::to_owned));
    columns
}

/// An age a claim is at, or what an unpaid claim says.
#[must_use]
pub fn age_cell(age: Option<u8>) -> String {
    age.map_or_else(|| NO_CLAIM.to_owned(), |age| age.to_string())
}

/// The person's `social-security` income, where they have one.
#[must_use]
pub fn benefit<'a>(plan: &'a Plan, person: &str) -> Option<&'a Income> {
    plan.income
        .iter()
        .find(|income| income.is_benefit_of(person))
}

/// Whether the person's benefit is computed and so has a claim to hold.
#[must_use]
pub fn is_claimed(plan: &Plan, person: &Person) -> bool {
    benefit(plan, &person.id).is_some_and(|income| income.amount.is_none())
}

/// The monthly figure typed for the person's benefit, where one is.
#[must_use]
pub fn typed_monthly(plan: &Plan, person: &Person) -> Option<Dollars> {
    Some(benefit(plan, &person.id)?.amount? / Dollars::from(MONTHS_PER_YEAR))
}

/// A person's row of the People table: their name, record, income, and
/// the monthly benefit estimated at 62, full retirement age and 70.
#[must_use]
pub fn person_row(
    plan: &Plan,
    person: &Person,
    is_held: bool,
    estimates: [Option<Dollars>; 3],
) -> Vec<String> {
    let record = match person.earnings.len() {
        0 => "none".to_owned(),
        years => format!("{years}y"),
    };
    let income = match benefit(plan, &person.id).map(|income| income.amount) {
        _ if is_held => HELD,
        None => "none",
        Some(None) => "computed",
        Some(Some(_)) => "typed",
    };
    let names = [person.display_name().to_owned(), record, income.to_owned()];
    let estimates =
        estimates.map(|figure| figure.map_or_else(|| NO_ESTIMATE.to_owned(), compact_money));
    names.into_iter().chain(estimates).collect()
}

/// What can be done for a person on the SSA Benefits page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum PersonAction {
    /// Record a statement's earnings on them.
    Import,
    /// Record a career at the salary the plan pays them.
    FillCareer,
    /// Drop their benefit's typed figure, so it is computed.
    ComputeBenefit,
    /// Keep their claim as the plan states it while the others' are searched.
    Hold,
    /// Search their claim again.
    LetVary,
    /// Empty their earnings record.
    ClearRecord,
    /// Take their Social Security income out of the plan.
    RemoveBenefit,
}

impl PersonAction {
    /// Every action, in the order they are offered.
    pub const ALL: [Self; 7] = [
        Self::Import,
        Self::FillCareer,
        Self::ComputeBenefit,
        Self::Hold,
        Self::LetVary,
        Self::ClearRecord,
        Self::RemoveBenefit,
    ];

    /// What the action is called where it is offered.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Import => "Import statement…",
            Self::FillCareer => "Estimate from salary",
            Self::ComputeBenefit => "Compute from record",
            Self::Hold => "Hold claim",
            Self::LetVary => "Let claim vary",
            Self::ClearRecord => "Clear record…",
            Self::RemoveBenefit => "Remove Social Security…",
        }
    }

    /// Whether `person` has a use for it, given whether their claim is held.
    #[must_use]
    pub fn is_offered(self, plan: &Plan, person: &Person, is_held: bool) -> bool {
        match self {
            Self::Import => true,
            Self::FillCareer => typed_monthly(plan, person).is_none(),
            Self::ComputeBenefit => typed_monthly(plan, person).is_some(),
            Self::Hold => !is_held && is_claimed(plan, person),
            Self::LetVary => is_held,
            Self::ClearRecord => !person.earnings.is_empty(),
            Self::RemoveBenefit => benefit(plan, &person.id).is_some(),
        }
    }

    /// What is asked of `name` before it is done, where it drops something.
    #[must_use]
    pub fn question(self, name: &str) -> Option<String> {
        match self {
            Self::ClearRecord => Some(format!("Clear {name}'s earnings record?")),
            Self::RemoveBenefit => Some(format!("Remove {name}'s Social Security income?")),
            _ => None,
        }
    }

    /// The answer that does it, where it is asked about.
    #[must_use]
    pub const fn answer(self) -> Option<&'static str> {
        match self {
            Self::ClearRecord => Some("Clear"),
            Self::RemoveBenefit => Some("Remove"),
            _ => None,
        }
    }

    /// Makes the edit to the person `id` in `plan`, answering what it did;
    /// `None` for an action that is not an edit of the plan.
    ///
    /// # Errors
    ///
    /// Why the edit was refused: the person has no use for it.
    pub fn apply(
        self,
        plan: &mut Plan,
        tables: &TaxTables,
        id: &str,
    ) -> Option<Result<String, String>> {
        match self {
            Self::FillCareer => Some(fill_career(plan, tables, id)),
            Self::ComputeBenefit => Some(compute_benefit(plan, id)),
            Self::ClearRecord => Some(clear_record(plan, id)),
            Self::RemoveBenefit => Some(remove_benefit(plan, id)),
            Self::Import | Self::Hold | Self::LetVary => None,
        }
    }
}

/// What holding or letting go of `name`'s claim says, `is_held` the claim
/// as it now is.
#[must_use]
pub fn hold_said(name: &str, is_held: bool) -> String {
    if is_held {
        format!("{name}'s claim is held as the plan states it")
    } else {
        format!("{name}'s claim is searched again")
    }
}

/// Records a career at the salary `plan` pays the person `id`, where they
/// have no record, answering what it did.
fn fill_career(plan: &mut Plan, tables: &TaxTables, id: &str) -> Result<String, String> {
    let at = place_of(plan, id)?;
    let name = plan.person_name(id).to_owned();
    if !plan.household.people[at].earnings.is_empty() {
        return Err(format!(
            "{name} has an earnings record; a statement replaces it"
        ));
    }
    let career = career_at_salary(plan, tables, id)?;
    let years = career.len();
    plan.household.people[at].earnings = career;
    Ok(format!(
        "estimated {years} year(s) of earnings for {name} from a career at their salary"
    ))
}

/// Drops the typed figure of the person `id`'s benefit, so it is computed
/// from their record, answering what it did.
fn compute_benefit(plan: &mut Plan, id: &str) -> Result<String, String> {
    let name = plan.person_name(id).to_owned();
    let typed =
        (plan.income.iter()).position(|income| income.is_benefit_of(id) && income.amount.is_some());
    let Some(at) = typed else {
        return Err(format!("{name} has no typed benefit to compute"));
    };
    plan.income[at].amount = None;
    Ok(format!("{name}'s benefit is computed from their record"))
}

/// Empties the person `id`'s earnings record, answering what it did.
fn clear_record(plan: &mut Plan, id: &str) -> Result<String, String> {
    let at = place_of(plan, id)?;
    let name = plan.person_name(id).to_owned();
    let earnings = &mut plan.household.people[at].earnings;
    if earnings.is_empty() {
        return Err(format!("{name} has no earnings record"));
    }
    earnings.clear();
    Ok(format!("cleared {name}'s earnings record"))
}

/// Takes the person `id`'s `social-security` income out of `plan`,
/// answering what it did.
fn remove_benefit(plan: &mut Plan, id: &str) -> Result<String, String> {
    let name = plan.person_name(id).to_owned();
    let Some(at) = plan
        .income
        .iter()
        .position(|income| income.is_benefit_of(id))
    else {
        return Err(format!("{name} has no Social Security income"));
    };
    plan.income.remove(at);
    Ok(format!("removed {name}'s Social Security income"))
}

fn place_of(plan: &Plan, id: &str) -> Result<usize, String> {
    let people = &plan.household.people;
    people
        .iter()
        .position(|person| person.id == id)
        .ok_or_else(|| NOBODY.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::EXAMPLES;

    fn starter() -> Plan {
        Plan::from_toml_str(EXAMPLES[0].2).expect("the plan parses")
    }

    fn claim(owner: &str, age: u8) -> Claim {
        Claim {
            income: format!("ss-{owner}"),
            owner: owner.to_owned(),
            age,
        }
    }

    #[test]
    fn claims_are_said_by_name() {
        let plan = starter();
        let person = &plan.household.people[0];
        let claims = [claim(&person.id, 70)];
        let name = person.display_name();
        assert_eq!(
            take_question(&plan, &claims),
            format!("Take these claims? {name} at 70.")
        );
        assert_eq!(taken(&plan, &claims), format!("claimed {name} at 70"));
    }

    #[test]
    fn a_record_is_filled_only_where_there_is_none_and_cleared_once() {
        let mut plan = starter();
        let tables = TaxTables::embedded();
        let id = plan.household.people[0].id.clone();
        plan.household.people[0].earnings.clear();
        assert!(clear_record(&mut plan, &id).is_err(), "no record to clear");
        let said = fill_career(&mut plan, &tables, &id).expect("filled");
        assert!(said.starts_with("estimated "), "{said}");
        assert!(!plan.household.people[0].earnings.is_empty());
        assert!(fill_career(&mut plan, &tables, &id).is_err(), "a record");
        clear_record(&mut plan, &id).expect("cleared");
        assert!(plan.household.people[0].earnings.is_empty());
    }

    #[test]
    fn a_typed_benefit_is_computed_and_a_benefit_removed() {
        let mut plan = starter();
        let id = plan.household.people[0].id.clone();
        let at = (plan.income.iter())
            .position(|income| income.is_benefit_of(&id))
            .expect("the starter has a benefit");
        plan.income[at].amount = Some(24_000);
        let person = plan.household.people[0].clone();
        assert!(PersonAction::ComputeBenefit.is_offered(&plan, &person, false));
        assert!(!PersonAction::Hold.is_offered(&plan, &person, false));
        compute_benefit(&mut plan, &id).expect("computed");
        assert!(benefit(&plan, &id).is_some_and(|income| income.amount.is_none()));
        assert!(compute_benefit(&mut plan, &id).is_err(), "nothing typed");
        assert!(PersonAction::Hold.is_offered(&plan, &person, false));
        remove_benefit(&mut plan, &id).expect("removed");
        assert!(benefit(&plan, &id).is_none());
        assert!(remove_benefit(&mut plan, &id).is_err());
    }

    #[test]
    fn a_row_says_a_held_claim_and_a_missing_estimate() {
        let plan = starter();
        let person = &plan.household.people[0];
        let row = person_row(&plan, person, true, [None, Some(2_000), None]);
        assert_eq!(row[2], HELD);
        assert_eq!(row[3], NO_ESTIMATE);
        assert_eq!(row.len(), PEOPLE_COLUMNS.len());
    }
}
