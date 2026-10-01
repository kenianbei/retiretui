//! The withdrawal-order search as every surface says it: an order in
//! words, the options' columns, and what is asked and said when one is
//! taken into the plan.

use retiretui_engine::optimize::OrderSearch;
use retiretui_engine::plan::TreatmentClass;
use retiretui_engine::project::Projection;

use super::overview::{beats, gain};
use crate::present;

/// What the options say before anything is searched.
pub const NOTHING_SEARCHED: &str = "Every order the plan's kinds of account can be withdrawn from is ranked here as soon as the plan is valid.";

/// What the tool is for, in a line.
pub const ABOUT: &str = "The kinds of account the plan withdraws from when income falls short, in every order: which is spent first changes what is taxed, and when.";

/// The column an option's order is under.
pub const ORDER: &str = "Order";

/// What the Overview says where no order beats the plan's own.
pub const ORDER_AS_PLANNED: &str = "Withdrawal order as planned is best";

/// The options' columns: the order, what an option ends with against the
/// plan, then the figures.
#[must_use]
pub fn option_columns() -> Vec<&'static str> {
    [ORDER, super::AGAINST_PLAN]
        .into_iter()
        .chain(super::FIGURES)
        .collect()
}

/// A class as it is said after the first word of a sentence.
fn within(class: TreatmentClass) -> String {
    let word = present::treatment_class(class);
    match class {
        TreatmentClass::Taxable | TreatmentClass::Deferred => word.to_lowercase(),
        TreatmentClass::Roth | TreatmentClass::Hsa => word.to_owned(),
    }
}

/// An order as a sentence says it inside itself: `deferred, taxable, Roth`.
fn said_within(order: &[TreatmentClass]) -> String {
    let each: Vec<String> = order.iter().map(|&class| within(class)).collect();
    each.join(", ")
}

/// An order as a row says it: `Deferred, taxable, Roth`.
#[must_use]
pub fn said(order: &[TreatmentClass]) -> String {
    let Some((&first, rest)) = order.split_first() else {
        return String::new();
    };
    std::iter::once(present::treatment_class(first).to_owned())
        .chain(rest.iter().map(|&class| within(class)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What is asked before `order` is taken into the plan.
#[must_use]
pub fn take_question(order: &[TreatmentClass]) -> String {
    format!("Withdraw in this order? {}.", said(order))
}

/// What is said once `order` is taken into the plan.
#[must_use]
pub fn taken(order: &[TreatmentClass]) -> String {
    format!("now withdrawing in the order {}", said_within(order))
}

/// What the Overview's row says of `search` against `current`: the best
/// order and its gain where it beats the plan's own.
#[must_use]
pub fn order_said(search: &OrderSearch, current: &Projection, nominal: bool) -> String {
    let Some(best) = search.candidates.first() else {
        return ORDER_AS_PLANNED.to_owned();
    };
    if !beats(&best.projection, current) {
        return ORDER_AS_PLANNED.to_owned();
    }
    let gain = gain(&best.projection, current, nominal);
    format!("Withdraw in the order {}: {gain}", said_within(&best.order))
}

#[cfg(test)]
mod tests {
    use retiretui_engine::market::Progress;
    use retiretui_engine::optimize::optimize_order;
    use retiretui_engine::params::TaxTables;
    use retiretui_engine::plan::Plan;
    use retiretui_engine::plan::TreatmentClass::{Deferred, Hsa, Roth, Taxable};

    use super::*;
    use crate::setup::EXAMPLES;

    #[test]
    fn an_order_is_said_as_a_list_whose_first_word_is_capitalised() {
        assert_eq!(said(&[Deferred, Taxable, Roth]), "Deferred, taxable, Roth");
        assert_eq!(
            said(&[Hsa, Roth, Deferred, Taxable]),
            "HSA, Roth, deferred, taxable"
        );
        assert_eq!(said(&[]), "");
        assert_eq!(
            take_question(&[Taxable, Roth]),
            "Withdraw in this order? Taxable, Roth."
        );
        assert_eq!(
            taken(&[Taxable, Roth]),
            "now withdrawing in the order taxable, Roth"
        );
        assert_eq!(
            option_columns(),
            [
                "Order",
                "Vs. the plan",
                "Unfunded",
                "Ends with",
                "Taxes",
                "Medicare"
            ]
        );
    }

    fn searched(name: &str) -> OrderSearch {
        let (_, _, text) = EXAMPLES
            .iter()
            .find(|(slug, ..)| *slug == name)
            .expect("the example exists");
        let plan = Plan::from_toml_str(text).expect("the plan parses");
        optimize_order(&plan, &TaxTables::embedded(), &Progress::default()).expect("searched")
    }

    #[test]
    fn the_overview_says_the_best_order_only_where_it_beats_the_plans_own() {
        let better = searched("early-retiree.toml");
        assert_eq!(
            order_said(&better, &better.baseline, false),
            format!(
                "Withdraw in the order deferred, taxable, Roth, HSA: {}",
                gain(&better.best().projection, &better.baseline, false)
            )
        );
        let planned = searched("starter.toml");
        assert_eq!(
            order_said(&planned, &planned.baseline, false),
            ORDER_AS_PLANNED
        );
    }
}
