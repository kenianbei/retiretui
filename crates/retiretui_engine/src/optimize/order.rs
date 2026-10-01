//! The withdrawal-order optimizer: the classes a plan's order lists are
//! tried in every order they can be drained in, and the orders ranked by
//! what the household ends with.

use serde::Serialize;

use crate::params::TaxTables;
use crate::plan::{Issue, Plan, PlanError, SCHEMA_VERSION, TreatmentClass};
use crate::project::{Projection, project};
use crate::search::{Progress, RunError, run_all};

/// The fewest classes there is an order of.
const FEWEST_ORDERED: usize = 2;

/// One order and the plan projected under it.
#[derive(Debug, Clone)]
pub struct OrderCandidate {
    /// Of the orders that project alike, the plan's own, or else the first
    /// tried: the one that keeps the most of the plan's order from its front.
    pub order: Vec<TreatmentClass>,
    /// The plan projected draining in that order.
    pub projection: Projection,
}

/// Every order searched, ranked, beside the plan as stated.
#[derive(Debug, Clone)]
pub struct OrderSearch {
    /// The plan projected under its own order.
    pub baseline: Projection,
    /// One candidate for each distinct projection, best first: least
    /// unfunded spending, then the highest final net worth in today's
    /// dollars, then the plan's own order, then the nearest to it.
    pub candidates: Vec<OrderCandidate>,
}

impl OrderSearch {
    /// The best-ranked candidate.
    ///
    /// # Panics
    ///
    /// On a search built by hand with no candidate; [`optimize_order`]
    /// never returns one.
    #[must_use]
    pub fn best(&self) -> &OrderCandidate {
        &self.candidates[0]
    }
}

/// Tries the classes the plan's order lists in every order, and ranks what
/// each projects to. A class the plan leaves out stays out. Accounts with a
/// `drain_priority` drain ahead of every class, so a listed class holding no
/// other account is not ordered: it keeps its place after the ones that are.
///
/// # Errors
///
/// [`RunError::Refused`] when fewer than two listed classes hold an account
/// to order; [`RunError::Cancelled`] when `progress` is cancelled before
/// every order is projected.
pub fn optimize_order(
    plan: &Plan,
    tables: &TaxTables,
    progress: &Progress,
) -> Result<OrderSearch, RunError> {
    let orders = orders_of(plan).ok_or_else(|| {
        RunError::Refused(vec![Issue {
            path: "plan.withdrawal_order".to_owned(),
            message:
                "fewer than two of the listed classes hold an account, so there is nothing to order"
                    .to_owned(),
        }])
    })?;
    let projections = run_all(orders.len(), progress, |at| {
        let mut ordered = plan.clone();
        apply_order(&mut ordered, &orders[at]);
        Some(project(&ordered, tables))
    })?;
    let baseline = projections[0].clone();
    let mut candidates: Vec<OrderCandidate> = Vec::new();
    for (order, projection) in orders.into_iter().zip(projections) {
        if !candidates.iter().any(|kept| kept.projection == projection) {
            candidates.push(OrderCandidate { order, projection });
        }
    }
    candidates.sort_by_cached_key(|candidate| super::rank_key(&candidate.projection));
    Ok(OrderSearch {
        baseline,
        candidates,
    })
}

/// The plan's own order, then every order of the classes it lists that hold
/// an account without a `drain_priority`, each followed by the listed
/// classes that hold none; nothing where there are not two to order.
fn orders_of(plan: &Plan) -> Option<Vec<Vec<TreatmentClass>>> {
    let stated = &plan.plan.withdrawal_order;
    let is_ordered = |class: &TreatmentClass| {
        plan.accounts
            .iter()
            .any(|account| account.drain_priority.is_none() && account.treatment() == *class)
    };
    let (ordered, trailing): (Vec<TreatmentClass>, Vec<TreatmentClass>) =
        stated.iter().partition(|&class| is_ordered(class));
    if ordered.len() < FEWEST_ORDERED {
        return None;
    }
    let mut orders = vec![stated.clone()];
    for mut order in permutations(&ordered) {
        order.extend_from_slice(&trailing);
        if order != *stated {
            orders.push(order);
        }
    }
    Some(orders)
}

/// Every arrangement of `classes`, the one given first and each after it
/// keeping as much of the given order from its front as is left to keep.
fn permutations(classes: &[TreatmentClass]) -> Vec<Vec<TreatmentClass>> {
    if classes.len() <= 1 {
        return vec![classes.to_vec()];
    }
    let mut arranged = Vec::new();
    for (at, &first) in classes.iter().enumerate() {
        let mut rest = classes.to_vec();
        rest.remove(at);
        for mut tail in permutations(&rest) {
            tail.insert(0, first);
            arranged.push(tail);
        }
    }
    arranged
}

/// What an order does to a plan: its `withdrawal_order` restated.
pub fn apply_order(plan: &mut Plan, order: &[TreatmentClass]) {
    plan.plan.withdrawal_order = order.to_vec();
}

/// The scenario overlay for an order, as canonical TOML: `base` plus a
/// `[plan]` table stating `withdrawal_order`.
///
/// # Errors
///
/// Returns [`PlanError::Serialize`] when serialization fails.
pub fn order_overlay(base: &str, order: &[TreatmentClass]) -> Result<String, PlanError> {
    #[derive(Serialize)]
    struct Restated<'a> {
        withdrawal_order: &'a [TreatmentClass],
    }
    #[derive(Serialize)]
    struct OverlayDocument<'a> {
        schema: u32,
        base: &'a str,
        plan: Restated<'a>,
    }
    Ok(toml::to_string_pretty(&OverlayDocument {
        schema: SCHEMA_VERSION,
        base,
        plan: Restated {
            withdrawal_order: order,
        },
    })?)
}
