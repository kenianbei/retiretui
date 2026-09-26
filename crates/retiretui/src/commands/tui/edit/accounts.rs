//! The Accounts domain. What an account earns is stated one of three
//! ways - a fixed return, one mix of asset classes, or a glide path of
//! mixes - which a pick no file holds chooses between.

use retiretui_engine::plan::{Account, Plan};
use toml::{Table, Value};

use super::applies;
use super::cells::Column;
use super::codec::left_of;
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
const SHARE_HELP: &str = "Its share of the account, rebalanced every year.";
const CASH_HELP: &str = "Whatever stocks and bonds leave, held in cash.";
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
                steps.retain(is_filled_step);
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

const CASH: &str = "cash";

/// Holds in cash what a mix's stocks and bonds leave of the whole.
fn fill_cash(mix: &mut Table) {
    let left = left_of(mix, CASH);
    if left == 0.0 {
        mix.remove(CASH);
    } else {
        mix.insert(CASH.to_owned(), Value::Float(left));
    }
}

/// Whether a glide step holds anything; one that does not is dropped.
fn is_filled_step(step: &Value) -> bool {
    step.as_table().is_some_and(|step| !step.is_empty())
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

/// Step `STEP` of a glide path is offered while it or a later step holds
/// anything, and after the last that does: every filled step, then one
/// blank one.
fn step_shown<const STEP: usize>(item: &Table) -> bool {
    let steps = || {
        item.get(ALLOCATION)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
    };
    glides(item) && (STEP == 0 || steps().skip(STEP - 1).any(is_filled_step))
}

const fn share(key: &'static str, label: &'static str, shown: fn(&Table) -> bool) -> FieldSpec {
    FieldSpec::share(key, label)
        .shown_when(shown)
        .help(SHARE_HELP)
}

const fn cash(key: &'static str, label: &'static str, shown: fn(&Table) -> bool) -> FieldSpec {
    FieldSpec::remainder(key, label)
        .shown_when(shown)
        .help(CASH_HELP)
}

const fn step_from(key: &'static str, label: &'static str, shown: fn(&Table) -> bool) -> FieldSpec {
    FieldSpec::trigger(key, label)
        .shown_when(shown)
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
        cash("allocation.cash", "Cash", holds_mix),
        step_from("allocation.0.from", "Mix 1 from", step_shown::<0>),
        share("allocation.0.stocks", "  Stocks", step_shown::<0>),
        share("allocation.0.bonds", "  Bonds", step_shown::<0>),
        cash("allocation.0.cash", "  Cash", step_shown::<0>),
        step_from("allocation.1.from", "Mix 2 from", step_shown::<1>),
        share("allocation.1.stocks", "  Stocks", step_shown::<1>),
        share("allocation.1.bonds", "  Bonds", step_shown::<1>),
        cash("allocation.1.cash", "  Cash", step_shown::<1>),
        step_from("allocation.2.from", "Mix 3 from", step_shown::<2>),
        share("allocation.2.stocks", "  Stocks", step_shown::<2>),
        share("allocation.2.bonds", "  Bonds", step_shown::<2>),
        cash("allocation.2.cash", "  Cash", step_shown::<2>),
        step_from("allocation.3.from", "Mix 4 from", step_shown::<3>),
        share("allocation.3.stocks", "  Stocks", step_shown::<3>),
        share("allocation.3.bonds", "  Bonds", step_shown::<3>),
        cash("allocation.3.cash", "  Cash", step_shown::<3>),
        step_from("allocation.4.from", "Mix 5 from", step_shown::<4>),
        share("allocation.4.stocks", "  Stocks", step_shown::<4>),
        share("allocation.4.bonds", "  Bonds", step_shown::<4>),
        cash("allocation.4.cash", "  Cash", step_shown::<4>),
        step_from("allocation.5.from", "Mix 6 from", step_shown::<5>),
        share("allocation.5.stocks", "  Stocks", step_shown::<5>),
        share("allocation.5.bonds", "  Bonds", step_shown::<5>),
        cash("allocation.5.cash", "  Cash", step_shown::<5>),
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
        let whole_number = written("allocation = { stocks = 1 }", MIX);
        assert!(
            get_path(&whole_number, "allocation.cash").is_none(),
            "{whole_number}"
        );
    }

    #[test]
    fn a_glide_path_drops_its_blank_steps_and_keeps_those_the_form_does_not_show() {
        let step = |age: u8| format!("{{ from = {{ age = {age} }}, stocks = 0.5 }}");
        let mut steps: Vec<String> = (50..57).map(step).collect();
        steps.insert(1, "{}".to_owned());
        let steps = written(&format!("allocation = [{}]", steps.join(", ")), GLIDE);
        let steps = steps["allocation"].as_array().unwrap();
        assert_eq!(steps.len(), 7, "{steps:?}");
        assert_eq!(
            steps[6]["cash"].as_float(),
            Some(0.5),
            "the seventh step, past the form's last, filled"
        );
    }

    #[test]
    fn each_filled_step_is_offered_and_one_blank_one_after_the_last() {
        let glide = |text: &str| {
            let mut glide = item(text);
            glide.insert(INVESTED.to_owned(), Value::String(GLIDE.to_owned()));
            glide
        };
        let empty = glide("");
        assert!(step_shown::<0>(&empty) && !step_shown::<1>(&empty));
        let second_blanked =
            glide("allocation = [{ from = { age = 50 } }, {}, { from = { age = 70 } }]");
        assert!(
            step_shown::<1>(&second_blanked),
            "a later step holds it open"
        );
        assert!(step_shown::<3>(&second_blanked) && !step_shown::<4>(&second_blanked));
        assert!(
            !step_shown::<0>(&item("")),
            "not while the pick is a return"
        );
    }

    #[test]
    fn a_fixed_return_or_a_change_of_shape_leaves_no_allocation_behind() {
        assert!(!written("allocation = { stocks = 1.0 }", FIXED).contains_key(ALLOCATION));
        assert!(!written("allocation = { stocks = 1.0 }", GLIDE).contains_key(ALLOCATION));
        assert!(!written("allocation = [{ from = { age = 60 } }]", MIX).contains_key(ALLOCATION));
    }
}
