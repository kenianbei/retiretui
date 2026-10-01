//! The withdrawal-order search as every surface says it: an order in
//! words, the options' columns, and what is asked and said when one is
//! taken into the plan.

use retiretui_engine::plan::TreatmentClass;

use crate::present;

/// What the options say before anything is searched.
pub const NOTHING_SEARCHED: &str = "Every order the plan's kinds of account can be withdrawn from is ranked here as soon as the plan is valid.";

/// What the tool is for, in a line.
pub const ABOUT: &str = "The kinds of account the plan withdraws from when income falls short, in every order: which is spent first changes what is taxed, and when.";

/// The column an option's order is under.
pub const ORDER: &str = "Order";

/// The options' columns: the order, what an option ends with against the
/// plan, then the figures.
#[must_use]
pub fn option_columns() -> Vec<&'static str> {
    [ORDER, super::AGAINST_PLAN]
        .into_iter()
        .chain(super::FIGURES)
        .collect()
}

/// An order as a sentence says it inside itself: `deferred, taxable, Roth`.
#[must_use]
pub fn said_within(order: &[TreatmentClass]) -> String {
    let each: Vec<String> = (order.iter())
        .map(|&class| {
            let word = present::treatment_class(class);
            match class {
                TreatmentClass::Taxable | TreatmentClass::Deferred => word.to_lowercase(),
                TreatmentClass::Roth | TreatmentClass::Hsa => word.to_owned(),
            }
        })
        .collect();
    each.join(", ")
}

/// An order as a row says it: `Deferred, taxable, Roth`.
#[must_use]
pub fn said(order: &[TreatmentClass]) -> String {
    let mut text = said_within(order);
    if let Some(first) = text.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    text
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

#[cfg(test)]
mod tests {
    use retiretui_engine::plan::TreatmentClass::{Deferred, Hsa, Roth, Taxable};

    use super::*;

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
}
