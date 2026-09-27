//! What the Overview asks is better than the plan: each Roth owner's best
//! ladder, the household's best claims, and the plan from every historical
//! start, searched as the tools search them.

use std::collections::BTreeSet;

use retiretui_engine::market::{History, Progress, Runs, historical};
use retiretui_engine::optimize::{ClaimSearch, optimize_claims, rank_key};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Plan, TreatmentClass};
use retiretui_engine::project::Projection;

use super::ladders::Swept;
use crate::present::signed_money;

/// A plan, the people whose claims are held as it states them, and the
/// conversion answers held but the destination.
pub type Searched = (Plan, BTreeSet<String>, toml::Table);

/// What the Overview's searches found.
pub struct Found {
    /// The plan from every start year, where it could be run.
    pub historical: Option<Runs>,
    /// The claim search, where anything computes a benefit.
    pub claims: Option<ClaimSearch>,
    /// Each Roth owner's best ladder.
    pub ladders: Vec<Ladder>,
}

/// A Roth owner's ladders into their Roth account, `None` where the search
/// is refused under the page's answers.
pub struct Ladder {
    /// Whose Roth account it fills.
    pub owner: String,
    /// The account.
    pub destination: String,
    /// What the search found.
    pub swept: Option<Swept>,
}

/// Every search, the cheapest first; none once cancelled.
pub fn search(
    (plan, held, answers): &Searched,
    (tables, history): (&TaxTables, &History),
    progress: &Progress,
) -> Option<Found> {
    let historical = historical(plan, tables, history, progress).ok();
    let held: Vec<String> = held.iter().cloned().collect();
    let claims = optimize_claims(plan, tables, &[], &held, progress).ok();
    let mut ladders = Vec::new();
    for owner in roth_owners(plan) {
        if progress.is_cancelled() {
            return None;
        }
        ladders.push(best_ladder(plan, tables, answers, owner, progress));
    }
    if progress.is_cancelled() {
        return None;
    }
    Some(Found {
        historical,
        claims,
        ladders,
    })
}

/// The best ladder into `owner`'s Roth account, searched as the Roth
/// Conversions page searches under the `answers` it holds.
fn best_ladder(
    plan: &Plan,
    tables: &TaxTables,
    answers: &toml::Table,
    (owner, destination): (&str, &str),
    progress: &Progress,
) -> Ladder {
    Ladder {
        owner: owner.to_owned(),
        destination: destination.to_owned(),
        swept: super::ladders::sweep_into(plan, tables, answers, destination, progress),
    }
}

/// Each person with a Roth account, and the first they own.
fn roth_owners(plan: &Plan) -> impl Iterator<Item = (&str, &str)> {
    plan.household.people.iter().filter_map(|person| {
        let roth = plan.accounts.iter().find(|account| {
            account.owner == person.id && account.treatment() == TreatmentClass::Roth
        })?;
        Some((person.id.as_str(), roth.id.as_str()))
    })
}

/// Whether `option` ranks ahead of `current`, as the tools rank options.
#[must_use]
pub fn beats(option: &Projection, current: &Projection) -> bool {
    rank_key(option) < rank_key(current)
}

/// What `option` ends with against `current`, and leaves unfunded where
/// that differs.
#[must_use]
pub fn gain(option: &Projection, current: &Projection, nominal: bool) -> String {
    let (own, base) = (option.summary(!nominal), current.summary(!nominal));
    let ends = signed_money(own.final_net_worth - base.final_net_worth);
    let unfunded = own.lifetime_unfunded - base.lifetime_unfunded;
    if unfunded == 0 {
        format!("ends {ends}")
    } else {
        format!("ends {ends}, unfunded {}", signed_money(unfunded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::EXAMPLES;

    /// A plan whose historical runs are refused without a look at the
    /// progress, so only the checks between the steps can stop it.
    #[test]
    fn a_cancelled_search_stops_before_the_next_step() {
        let (_, _, starter) = EXAMPLES[0];
        let refused = format!("{starter}\n[market.historical]\nfrom = 1800\n");
        let searched = (
            Plan::from_toml_str(&refused).expect("the plan parses"),
            BTreeSet::new(),
            toml::Table::new(),
        );
        let tables = (&TaxTables::embedded(), History::embedded());
        let progress = Progress::default();
        let found = search(&searched, tables, &progress).expect("searched");
        assert!(found.historical.is_none(), "the runs were refused");
        progress.cancel();
        assert!(search(&searched, tables, &progress).is_none());
    }

    #[test]
    fn a_projection_does_not_beat_itself_and_gains_nothing() {
        let (_, _, starter) = EXAMPLES[0];
        let plan = Plan::from_toml_str(starter).expect("the plan parses");
        let projection = retiretui_engine::project::project(&plan, &TaxTables::embedded());
        assert!(!beats(&projection, &projection));
        assert_eq!(gain(&projection, &projection, false), "ends $0");
    }
}
