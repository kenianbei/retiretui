//! What the Overview asks is better than the plan: each Roth owner's best
//! ladder, the household's best claims and the best order to withdraw in,
//! and the plan from every historical start, searched as the tools search
//! them.

use std::collections::BTreeSet;

use retiretui_engine::market::{History, Progress, Runs, historical};
use retiretui_engine::optimize::{
    ClaimSearch, OrderSearch, optimize_claims, optimize_order, rank_key,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Plan, TreatmentClass};
use retiretui_engine::project::Projection;

use super::claims::said;
use super::ladders::{Swept, rate_label};
use super::orders::said_within;
use crate::present::{compact_money, signed_money};

/// The card's title.
pub const COULD_DO_BETTER: &str = "Could do better";

/// What a Roth owner's row says where no ladder beats the plan.
pub const NO_LADDER: &str = "no conversion ladder beats the plan";

/// What a Roth owner's row says where the search is refused.
pub const REFUSED: &str = "not searchable under the Roth Conversions answers";

/// What the claims row says where no claims beat the plan's own.
pub const CLAIMS_AS_PLANNED: &str = "Claims as planned are best";

/// What the order row says where no order beats the plan's own.
pub const ORDER_AS_PLANNED: &str = "Withdrawal order as planned is best";

/// What the card says where there is nothing to search.
pub const NOTHING_TO_SEARCH: &str = "No conversion, claim or withdrawal order to search";

/// A plan, the people whose claims are held as it states them, and the
/// conversion answers held but the destination.
pub type Searched = (Plan, BTreeSet<String>, toml::Table);

/// What the Overview's searches found.
pub struct Found {
    /// The plan from every start year, where it could be run.
    pub historical: Option<Runs>,
    /// The claim search, where anything computes a benefit.
    pub claims: Option<ClaimSearch>,
    /// The order search, where the plan withdraws from two classes or more.
    pub order: Option<OrderSearch>,
    /// Each Roth owner's best ladder.
    pub ladders: Vec<Ladder>,
}

/// A Roth owner's ladders into their Roth account, `None` where the search
/// is refused under the Roth Conversions answers.
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
    let order = optimize_order(plan, tables, progress).ok();
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
        order,
        ladders,
    })
}

/// The best ladder into `owner`'s Roth account, searched as the Roth
/// Conversions tool searches under the `answers` it holds.
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
pub fn roth_owners(plan: &Plan) -> impl Iterator<Item = (&str, &str)> {
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

/// What `option`, which [`beats`] `current`, ends with against it, and how
/// much more spending it covers in the basis shown, where it does: `beats`
/// ranks in today's dollars, so in future dollars it may not.
#[must_use]
pub fn gain(option: &Projection, current: &Projection, nominal: bool) -> String {
    let (own, base) = (option.summary(!nominal), current.summary(!nominal));
    let ends = signed_money(own.final_net_worth - base.final_net_worth);
    let covered = base.lifetime_unfunded - own.lifetime_unfunded;
    if covered <= 0 {
        format!("ends {ends}")
    } else {
        format!(
            "ends {ends}, covers {} more spending",
            compact_money(covered)
        )
    }
}

/// What a Roth owner's row says of `swept`, their ladders, against
/// `current`: the best bracket and its gain where it beats the plan.
#[must_use]
pub fn ladder_said(swept: Option<&Swept>, current: &Projection, nominal: bool) -> String {
    match swept.and_then(Swept::best) {
        None => REFUSED.to_owned(),
        Some(best) if beats(&best.optimized, current) => format!(
            "convert to {}, {}",
            rate_label(best.rate),
            gain(&best.optimized, current, nominal)
        ),
        Some(_) => NO_LADDER.to_owned(),
    }
}

/// What the claims row says of `search` over `plan` against `current`: the
/// best claims and their gain where they beat the plan's own.
#[must_use]
pub fn claims_said(
    plan: &Plan,
    search: &ClaimSearch,
    current: &Projection,
    nominal: bool,
) -> String {
    let Some(best) = search.candidates.first() else {
        return CLAIMS_AS_PLANNED.to_owned();
    };
    if !beats(&best.projection, current) {
        return CLAIMS_AS_PLANNED.to_owned();
    }
    let gain = gain(&best.projection, current, nominal);
    format!("Claim {}: {gain}", said(plan, &best.claims))
}

/// What the order row says of `search`: the best order and its gain where
/// it beats the plan's own.
#[must_use]
pub fn order_said(search: &OrderSearch, nominal: bool) -> String {
    let best = search.best();
    if !beats(&best.projection, &search.baseline) {
        return ORDER_AS_PLANNED.to_owned();
    }
    let gain = gain(&best.projection, &search.baseline, nominal);
    format!("Withdraw in the order {}: {gain}", said_within(&best.order))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::EXAMPLES;
    use crate::setup::examples::named;

    #[test]
    fn the_best_order_is_said_only_where_it_beats_the_plans_own() {
        let searched = |file: &str| {
            let (_, _, text) = named(file).expect("the example");
            let plan = Plan::from_toml_str(text).expect("the plan parses");
            optimize_order(&plan, &TaxTables::embedded(), &Progress::default()).expect("searched")
        };
        let better = searched("early-retiree.toml");
        assert_eq!(
            order_said(&better, false),
            format!(
                "Withdraw in the order deferred, taxable, Roth, HSA: {}",
                gain(&better.best().projection, &better.baseline, false)
            )
        );
        assert_eq!(
            order_said(&searched("starter.toml"), false),
            ORDER_AS_PLANNED
        );
    }

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

    #[test]
    fn an_option_leaving_less_uncovered_says_how_much_more_it_covers() {
        let (_, _, starter) = EXAMPLES[0];
        let spending = |amount: &str| {
            let text = starter.replacen("amount = 24000", amount, 1);
            let plan = Plan::from_toml_str(&text).expect("the plan parses");
            retiretui_engine::project::project(&plan, &TaxTables::embedded())
        };
        let (current, option) = (spending("amount = 240000"), spending("amount = 200000"));
        assert!(beats(&option, &current));
        let said = gain(&option, &current, false);
        let (own, base) = (option.summary(true), current.summary(true));
        let covered = compact_money(base.lifetime_unfunded - own.lifetime_unfunded);
        assert!(
            said.ends_with(&format!(", covers {covered} more spending")),
            "{said}"
        );
        assert!(!said.contains('-'), "{said}");
    }
}
