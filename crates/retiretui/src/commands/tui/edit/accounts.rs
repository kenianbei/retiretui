//! The Accounts domain. What an account earns is stated one of three
//! ways - a fixed return, one mix of asset classes, or a glide path of
//! mixes - which a pick no file holds chooses between.

use retiretui_engine::plan::{Account, Plan};
use toml::{Table, Value};

use super::applies;
use super::cells::Column;
use super::domain::{Domain, FieldSpec};
use super::offers::{RefSource, Vocabulary};
use crate::commands::tui::nav::Page;

/// The key of the pick that says how the account earns. No file holds
/// it: it is read from the shape of `allocation`, and written back as that
/// shape, whichever rows it leaves on show holding the rest.
const INVESTED: &str = "invested";
pub const FIXED: &str = "fixed";
pub const MIX: &str = "mix";
pub const GLIDE: &str = "glide";

const ALLOCATION: &str = "allocation";
const SHARE_HELP: &str =
    "Its share of the account, rebalanced every year. Cash is whatever stocks and bonds leave.";
const STEP_FROM_HELP: &str =
    "When this mix takes over. The first holds until the second fires; a blank step is dropped.";

fn seed_invested(item: &Table) -> Value {
    let form = match item.get(ALLOCATION) {
        None => FIXED,
        Some(Value::Array(_)) => GLIDE,
        Some(_) => MIX,
    };
    Value::String(form.to_owned())
}

/// Leaves `allocation` in the shape the pick names: none for a fixed
/// return, a table for one mix, and a list of steps for a glide path, its
/// blank steps dropped; each mix holding in cash what stocks and bonds
/// leave.
fn write_invested(item: &mut Table, form: &Value) {
    let shape = item.get(ALLOCATION).map(Value::is_array);
    match (form.as_str(), shape) {
        (Some(MIX), Some(false)) => {
            if let Some(Value::Table(mix)) = item.get_mut(ALLOCATION) {
                fill_cash(mix);
            }
        }
        (Some(GLIDE), Some(true)) => {
            if let Some(Value::Array(steps)) = item.get_mut(ALLOCATION) {
                steps.retain(|step| step.as_table().is_some_and(|step| !step.is_empty()));
                steps
                    .iter_mut()
                    .filter_map(Value::as_table_mut)
                    .for_each(fill_cash);
                if steps.is_empty() {
                    item.remove(ALLOCATION);
                }
            }
        }
        _ => {
            item.remove(ALLOCATION);
        }
    }
}

const STOCKS: &str = "stocks";
const BONDS: &str = "bonds";
const CASH: &str = "cash";

/// Below this, what stocks and bonds leave is rounding, not cash.
const NO_CASH: f64 = 1e-9;

/// Holds in cash what a mix's stocks and bonds leave of the whole.
fn fill_cash(mix: &mut Table) {
    let share = |key: &str| mix.get(key).and_then(Value::as_float).unwrap_or(0.0);
    let left = 1.0 - share(STOCKS) - share(BONDS);
    if left.abs() < NO_CASH {
        mix.remove(CASH);
    } else {
        mix.insert(CASH.to_owned(), Value::Float(left));
    }
}

fn invested(item: &Table) -> &str {
    item.get(INVESTED).and_then(Value::as_str).unwrap_or(FIXED)
}

fn earns_fixed(item: &Table) -> bool {
    invested(item) == FIXED
}

fn holds_mix(item: &Table) -> bool {
    invested(item) == MIX
}

fn glides(item: &Table) -> bool {
    invested(item) == GLIDE
}

const fn share(key: &'static str, label: &'static str, shown: fn(&Table) -> bool) -> FieldSpec {
    FieldSpec::share(key, label)
        .shown_when(shown)
        .help(SHARE_HELP)
}

const fn step_from(key: &'static str, label: &'static str) -> FieldSpec {
    FieldSpec::trigger(key, label)
        .shown_when(glides)
        .help(STEP_FROM_HELP)
}

pub struct Accounts;

impl Domain for Accounts {
    type Item = Account;
    const PAGE: Page = Page::Accounts;
    const PURPOSE: &'static str = "Savings, retirement accounts and investments";
    const PATH: &'static str = "accounts";
    const SINGULAR: &'static str = "Account";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::text("id", "ID")
            .help("A short unique handle other items refer to this account by."),
        FieldSpec::text("name", "Name").help("What the account is called. Blank shows the ID."),
        FieldSpec::choice("kind", "Type", Vocabulary::AccountKind)
            .help("The kind of account, which decides how it is taxed."),
        FieldSpec::flag("roth", "Roth")
            .shown_when(applies::supports_roth)
            .help("Taxed going in, tax-free coming out."),
        FieldSpec::refers("owner", "Owner", RefSource::Person)
            .help("Whose account it is."),
        FieldSpec::money("balance", "Balance").help("What it holds at the start of the plan."),
        FieldSpec::money("basis", "Basis")
            .shown_when(applies::keeps_basis)
            .help("What was already taxed: what a brokerage's holdings cost, blank meaning all of them; after-tax money in a deferred account, blank meaning none."),
        FieldSpec::choice(INVESTED, "Invested", Vocabulary::Holding)
            .derived(seed_invested, write_invested)
            .help("A fixed return every year, one mix of stocks, bonds and cash, or a mix that steps as you age. A mix's return follows the market."),
        FieldSpec::rate("expected_return", "Expected return")
            .shown_when(earns_fixed)
            .help("Growth per year, the same every year and in every market. Blank earns nothing."),
        share("allocation.stocks", "Stocks", holds_mix),
        share("allocation.bonds", "Bonds", holds_mix),
        step_from("allocation.0.from", "First mix from"),
        share("allocation.0.stocks", "  Stocks", glides),
        share("allocation.0.bonds", "  Bonds", glides),
        step_from("allocation.1.from", "Then from"),
        share("allocation.1.stocks", "  Stocks", glides),
        share("allocation.1.bonds", "  Bonds", glides),
        FieldSpec::trigger("locked_until", "Locked until")
            .blank("Never")
            .help("Until then, nothing can be withdrawn or transferred out."),
        FieldSpec::whole("drain_priority", "Drain priority").help(
            "Spend this account first; lower numbers go sooner. Blank follows the usual order.",
        ),
    ];
    const COLUMNS: &'static [Column] = &[
        Column::new("id").headed("Account"),
        Column::new("kind"),
        Column::new("owner"),
        Column::new("balance"),
    ];
    const BLANK: &'static str = r#"
kind = "cash"
owner = "{owner}"
balance = 0
"#;

    fn items(plan: &Plan) -> &[Account] {
        &plan.accounts
    }

    fn items_mut(plan: &mut Plan) -> &mut Vec<Account> {
        &mut plan.accounts
    }
}

#[cfg(test)]
mod tests {
    use super::super::codec::get_path;
    use super::*;

    fn item(text: &str) -> Table {
        text.parse().unwrap()
    }

    fn written(text: &str, form: &str) -> Table {
        let mut item = item(text);
        write_invested(&mut item, &Value::String(form.to_owned()));
        item
    }

    #[test]
    fn the_pick_is_read_from_the_shape_of_the_allocation() {
        let seeded = |text: &str| seed_invested(&item(text)).as_str().unwrap().to_owned();
        assert_eq!(seeded("expected_return = 0.05"), FIXED);
        assert_eq!(seeded("allocation = { stocks = 1.0 }"), MIX);
        assert_eq!(
            seeded("allocation = [{ from = { age = 60 }, stocks = 1.0 }]"),
            GLIDE
        );
    }

    #[test]
    fn a_mix_holds_in_cash_what_stocks_and_bonds_leave() {
        let mix = written("allocation = { stocks = 0.7, bonds = 0.2 }", MIX);
        let cash = get_path(&mix, "allocation.cash")
            .and_then(Value::as_float)
            .unwrap();
        assert!((cash - 0.1).abs() < 1e-12, "{mix}");
        let whole = written(
            "allocation = { stocks = 0.6, bonds = 0.4, cash = 0.2 }",
            MIX,
        );
        assert!(get_path(&whole, "allocation.cash").is_none(), "{whole}");
    }

    #[test]
    fn a_glide_path_drops_its_blank_steps_and_keeps_those_the_form_does_not_show() {
        let steps = written(
            "allocation = [{ from = { age = 50 }, stocks = 1.0 }, {}, { from = { age = 60 }, bonds = 1.0 }, { from = { age = 70 }, stocks = 0.5 }]",
            GLIDE,
        );
        let steps = steps["allocation"].as_array().unwrap();
        assert_eq!(steps.len(), 3, "{steps:?}");
        assert_eq!(
            steps[2]["cash"].as_float(),
            Some(0.5),
            "the third step, filled"
        );
    }

    #[test]
    fn a_fixed_return_or_a_change_of_shape_leaves_no_allocation_behind() {
        assert!(!written("allocation = { stocks = 1.0 }", FIXED).contains_key(ALLOCATION));
        assert!(!written("allocation = { stocks = 1.0 }", GLIDE).contains_key(ALLOCATION));
        assert!(!written("allocation = [{ from = { age = 60 } }]", MIX).contains_key(ALLOCATION));
    }
}
