//! The claim-age optimizer: every Social Security benefit the engine
//! computes is tried at each whole claim age, jointly across the household,
//! and the candidates are ranked by what the household ends with.

use serde::Serialize;

use crate::params::TaxTables;
use crate::plan::{
    ColaSpec, Income, IncomeKind, Issue, Plan, PlanError, SCHEMA_VERSION, Trigger, push_issue,
};
use crate::project::{Projection, horizon_year, project};
use crate::tax::{EARLIEST_CLAIM_AGE, LATEST_CREDIT_AGE};

/// What a made-up income's id starts with, before its owner's.
const ADDED_ID_PREFIX: &str = "ss-";

/// One income claimed at one age.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Claim {
    /// The income's id.
    pub income: String,
    /// The person whose age the claim is at.
    pub owner: String,
    /// The whole age claimed at.
    pub age: u8,
}

impl Claim {
    /// The income's `start` under this claim: the owner's age.
    #[must_use]
    pub fn trigger(&self) -> Trigger {
        Trigger {
            date: None,
            age: Some(self.age),
            owner: Some(self.owner.clone()),
            event: None,
            income: None,
            offset: None,
        }
    }
}

/// One cell of the grid: a claim per searched income and the projection
/// under them.
#[derive(Debug, Clone)]
pub struct ClaimCandidate {
    /// One claim per searched income, in the plan's income order.
    pub claims: Vec<Claim>,
    /// The plan projected with those claims.
    pub projection: Projection,
}

/// The grid searched, ranked, beside the plan as stated.
#[derive(Debug, Clone)]
pub struct ClaimSearch {
    /// The plan projected with its own claims.
    pub baseline: Projection,
    /// The age each searched income is claimed at in the plan as stated,
    /// in the order of `incomes`; `None` for one the plan does not pay.
    pub current: Vec<Option<u8>>,
    /// The ids searched, in the order every candidate's claims hold them.
    pub incomes: Vec<String>,
    /// The computed incomes the search made up for people with an earnings
    /// record and no `social-security` income; every candidate claims them.
    pub added: Vec<Income>,
    /// Every candidate, best first: least unfunded spending, then the
    /// highest final net worth in today's dollars, then the earlier claims.
    pub candidates: Vec<ClaimCandidate>,
}

impl ClaimSearch {
    /// The best-ranked candidate.
    ///
    /// # Panics
    ///
    /// On a search built by hand with no candidate; [`optimize_claims`]
    /// never returns one.
    #[must_use]
    pub fn best(&self) -> &ClaimCandidate {
        &self.candidates[0]
    }
}

/// Tries every searched income at each whole claim age the plan can still
/// reach - 62 or the owner's age at plan start, whichever is later, through
/// 70, inside the horizon - and ranks the grid. `incomes` names the incomes
/// to search; empty means every `social-security` income whose benefit is
/// computed, and a computed one made up for each person with an earnings
/// record and no `social-security` income - both leaving out the people
/// `held` names, whose claims stay as the plan states them.
///
/// # Errors
///
/// Returns validation issues when nothing computes a benefit, a named
/// income is unknown, not `social-security`, or states its amount, a
/// searched income has no id to be restated by, the id a made-up income
/// would take is held, or no claim age is left for an income.
pub fn optimize_claims(
    plan: &Plan,
    tables: &TaxTables,
    incomes: &[String],
    held: &[String],
) -> Result<ClaimSearch, Vec<Issue>> {
    let mut issues = Vec::new();
    let added = if incomes.is_empty() {
        added_incomes(plan, held, &mut issues)
    } else {
        Vec::new()
    };
    let mut extended = plan.clone();
    apply_claims(&mut extended, &added, &[]);
    let searched = searched_incomes(&extended, incomes, held, &mut issues);
    let grids: Vec<Grid> = searched
        .iter()
        .filter_map(|&index| grid_of(&extended, index, &mut issues))
        .collect();
    if !issues.is_empty() {
        return Err(issues);
    }
    let baseline = project(plan, tables);
    let mut working = extended.clone();
    let mut candidates: Vec<ClaimCandidate> = combinations(&grids)
        .iter()
        .map(|ages| candidate(&mut working, tables, &grids, ages))
        .collect();
    candidates.sort_by_cached_key(|candidate| {
        let ages: Vec<u8> = candidate.claims.iter().map(|claim| claim.age).collect();
        (super::rank_key(&candidate.projection), ages)
    });
    let current = grids
        .iter()
        .map(|grid| claimed_age(&baseline, plan, grid.id))
        .collect();
    Ok(ClaimSearch {
        baseline,
        current,
        incomes: grids.iter().map(|grid| grid.id.to_owned()).collect(),
        added,
        candidates,
    })
}

/// The age `income` is claimed at in `projection` of `plan`: its owner's
/// age in the first year it pays anything; `None` where it never does.
#[must_use]
fn claimed_age(projection: &Projection, plan: &Plan, income: &str) -> Option<u8> {
    let owner = plan.person(&plan.income_source(income)?.owner)?;
    let first = projection
        .years
        .iter()
        .find(|row| row.income.get(income).is_some_and(|&paid| paid > 0))?;
    u8::try_from(owner.age_in_year(first.year)).ok()
}

/// What a set of claims does to a plan: each of `added` it does not hold
/// yet appended, then every claimed income's `start` set to its claim.
pub fn apply_claims(plan: &mut Plan, added: &[Income], claims: &[Claim]) {
    for income in added {
        if !plan.income.iter().any(|held| held.id == income.id) {
            plan.income.push(income.clone());
        }
    }
    for claim in claims {
        let claimed = plan
            .income
            .iter_mut()
            .find(|income| income.id == claim.income);
        if let Some(income) = claimed {
            income.start = Some(claim.trigger());
        }
    }
}

/// The scenario overlay for a set of claims, as canonical TOML: `base` plus
/// one `[[income]]` fragment per claim - `id` and the `start` it moves to,
/// or the whole income where it is one of `added`, which the base lacks.
///
/// # Errors
///
/// Returns [`PlanError::Serialize`] when serialization fails.
pub fn claims_overlay(base: &str, added: &[Income], claims: &[Claim]) -> Result<String, PlanError> {
    #[derive(Serialize)]
    #[serde(untagged)]
    enum ClaimFragment<'a> {
        Restated { id: &'a str, start: Trigger },
        Added(Box<Income>),
    }
    #[derive(Serialize)]
    struct OverlayDocument<'a> {
        schema: u32,
        base: &'a str,
        income: Vec<ClaimFragment<'a>>,
    }
    let income = claims
        .iter()
        .map(|claim| {
            let made_up = added.iter().find(|income| income.id == claim.income);
            match made_up {
                Some(income) => ClaimFragment::Added(Box::new(Income {
                    start: Some(claim.trigger()),
                    ..income.clone()
                })),
                None => ClaimFragment::Restated {
                    id: &claim.income,
                    start: claim.trigger(),
                },
            }
        })
        .collect();
    Ok(toml::to_string_pretty(&OverlayDocument {
        schema: SCHEMA_VERSION,
        base,
        income,
    })?)
}

/// One searched income and the ages it can still be claimed at.
struct Grid<'a> {
    id: &'a str,
    owner: &'a str,
    ages: Vec<u8>,
}

/// A computed income for each person with an earnings record and no
/// `social-security` income, as `ss-<person>`.
fn added_incomes(plan: &Plan, held: &[String], issues: &mut Vec<Issue>) -> Vec<Income> {
    let has_benefit = |person: &str| {
        plan.income
            .iter()
            .any(|income| income.is_benefit_of(person))
    };
    let mut added = Vec::new();
    for person in &plan.household.people {
        if person.earnings.is_empty() || has_benefit(&person.id) || held.contains(&person.id) {
            continue;
        }
        let id = format!("{ADDED_ID_PREFIX}{}", person.id);
        if plan.income_source(&id).is_some() {
            push_issue(
                issues,
                "income",
                format!(
                    "`{id}` is taken, so {}'s benefit has no id to be added under",
                    person.id
                ),
            );
            continue;
        }
        added.push(computed_income(&person.id));
    }
    added
}

/// A `social-security` income of `owner`'s as `ss-<owner>`, its benefit
/// computed, with no claim yet: it validates only once a claim sets its
/// `start`.
pub(super) fn computed_income(owner: &str) -> Income {
    Income {
        id: format!("{ADDED_ID_PREFIX}{owner}"),
        name: None,
        kind: IncomeKind::SocialSecurity,
        owner: owner.to_owned(),
        amount: None,
        start: None,
        end: None,
        on: None,
        cola: ColaSpec::default(),
    }
}

/// The indices of the incomes to search: the named ones checked, or every
/// derived one.
fn searched_incomes(
    plan: &Plan,
    incomes: &[String],
    held: &[String],
    issues: &mut Vec<Issue>,
) -> Vec<usize> {
    if incomes.is_empty() {
        let is_searched = |income: &Income| income.is_derived() && !held.contains(&income.owner);
        let derived: Vec<usize> = (0..plan.income.len())
            .filter(|&index| is_searched(&plan.income[index]))
            .collect();
        if derived.is_empty() && issues.is_empty() {
            push_issue(
                issues,
                "income",
                "no social-security income computes its benefit, and no one without one has an earnings record",
            );
        }
        return derived;
    }
    let mut searched = Vec::with_capacity(incomes.len());
    for (i, id) in incomes.iter().enumerate() {
        let path = format!("options.incomes[{i}]");
        let Some(index) = plan.income.iter().position(|income| income.id == *id) else {
            push_issue(issues, path, format!("unknown income `{id}`"));
            continue;
        };
        if let Some(refusal) = refuse_named(&plan.income[index]) {
            push_issue(issues, path, refusal);
        } else if searched.contains(&index) {
            push_issue(issues, path, format!("`{id}` is named twice"));
        } else {
            searched.push(index);
        }
    }
    searched
}

fn refuse_named(income: &Income) -> Option<&'static str> {
    if income.kind != IncomeKind::SocialSecurity {
        Some("must be a social-security income")
    } else if income.amount.is_some() {
        Some("states its amount, so its benefit cannot move")
    } else {
        None
    }
}

fn grid_of<'a>(plan: &'a Plan, index: usize, issues: &mut Vec<Issue>) -> Option<Grid<'a>> {
    let income = &plan.income[index];
    let id = income.id.as_str();
    let owner = plan.person(&income.owner)?;
    let first = EARLIEST_CLAIM_AGE.max(owner.age_in_year(plan.plan.start_year));
    let horizon = horizon_year(plan);
    let ages: Vec<u8> = (first..=LATEST_CREDIT_AGE)
        .filter(|age| owner.birth.year() + age <= horizon)
        .map(|age| age as u8)
        .collect();
    if ages.is_empty() {
        push_issue(
            issues,
            format!("income[{index}].start"),
            format!("no claim age from {first} to {LATEST_CREDIT_AGE} falls inside the horizon"),
        );
        return None;
    }
    Some(Grid {
        id,
        owner: &owner.id,
        ages,
    })
}

/// Every way of picking one age per grid, in grid order.
fn combinations(grids: &[Grid<'_>]) -> Vec<Vec<u8>> {
    grids.iter().fold(vec![Vec::new()], |partial, grid| {
        partial
            .iter()
            .flat_map(|prefix| {
                grid.ages.iter().map(move |&age| {
                    let mut next = prefix.clone();
                    next.push(age);
                    next
                })
            })
            .collect()
    })
}

/// One cell projected: every searched income's `start` is overwritten on
/// `working`, which already holds the added incomes, so one copy of the plan
/// serves the whole grid.
fn candidate(
    working: &mut Plan,
    tables: &TaxTables,
    grids: &[Grid<'_>],
    ages: &[u8],
) -> ClaimCandidate {
    let claims: Vec<Claim> = grids
        .iter()
        .zip(ages)
        .map(|(grid, &age)| Claim {
            income: grid.id.to_owned(),
            owner: grid.owner.to_owned(),
            age,
        })
        .collect();
    apply_claims(working, &[], &claims);
    ClaimCandidate {
        claims,
        projection: project(working, tables),
    }
}
