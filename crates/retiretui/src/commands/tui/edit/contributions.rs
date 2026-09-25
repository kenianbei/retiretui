//! The Contributions domain: what is paid into each account, by whom, and
//! how much - stated one of four ways, which a pick no file holds chooses
//! between.

use retiretui_engine::plan::{Contribution, Plan};
use toml::{Table, Value};

use super::cells::Column;
use super::codec::get_path;
use super::domain::{Domain, FieldSpec, GROWTH_HELP};
use super::offers::{RefSource, Vocabulary};
use crate::commands::tui::nav::Page;
use crate::commands::tui::present;

/// The key of the pick that says which way the amount is stated. No file
/// holds it: it is read from which of the four keys the item states, and
/// written back as `max` alone, the other three being rows of their own.
pub const AMOUNT_AS: &str = "amount_as";
pub const DOLLARS: &str = "dollars";
pub const SHARE: &str = "share";
/// The word and the key are one: the pick's value is what is written.
pub const MAXIMUM: &str = "max";
pub const MATCH: &str = "match";

const AMOUNT_KEY: &str = "amount";
const RATE_KEY: &str = "rate";
const OF_KEY: &str = "of";
const MATCH_RATE_KEY: &str = "match.rate";
const MATCH_UP_TO_KEY: &str = "match.up_to";

fn seed_amount_form(item: &Table) -> Value {
    let form = if item.get(MAXIMUM).and_then(Value::as_bool) == Some(true) {
        MAXIMUM
    } else if item.contains_key(MATCH) {
        MATCH
    } else if item.contains_key(RATE_KEY) {
        SHARE
    } else {
        DOLLARS
    };
    Value::String(form.to_owned())
}

/// The maximum is the one form with no row of its own, so the pick states
/// it; the other three are what their rows hold.
fn write_amount_form(item: &mut Table, form: &Value) {
    if form.as_str() == Some(MAXIMUM) {
        item.insert(MAXIMUM.to_owned(), Value::Boolean(true));
    } else {
        item.remove(MAXIMUM);
    }
}

fn amount_form(item: &Table) -> &str {
    item.get(AMOUNT_AS)
        .and_then(Value::as_str)
        .unwrap_or(DOLLARS)
}

pub fn pays_dollars(item: &Table) -> bool {
    amount_form(item) == DOLLARS
}

pub fn pays_share(item: &Table) -> bool {
    amount_form(item) == SHARE
}

pub fn pays_match(item: &Table) -> bool {
    amount_form(item) == MATCH
}

pub fn names_income(item: &Table) -> bool {
    pays_share(item) || pays_match(item)
}

/// The amount as the table's cell says it, in whichever form it takes.
fn amount_phrase(item: &Table, plan: &Plan) -> String {
    let real = |key: &str| get_path(item, key).and_then(Value::as_float);
    let income = || {
        let of = get_path(item, OF_KEY).and_then(Value::as_str).unwrap_or("");
        present::income_name(plan, of).to_owned()
    };
    if item.get(MAXIMUM).and_then(Value::as_bool) == Some(true) {
        return "the maximum".to_owned();
    }
    if let (Some(rate), Some(up_to)) = (real(MATCH_RATE_KEY), real(MATCH_UP_TO_KEY)) {
        let (rate, up_to) = (present::rate(rate), present::rate(up_to));
        return format!("{rate} match up to {up_to} of {}", income());
    }
    if let Some(rate) = real(RATE_KEY) {
        return format!("{} of {}", present::rate(rate), income());
    }
    let amount = get_path(item, AMOUNT_KEY).and_then(Value::as_integer);
    amount.map(present::money).unwrap_or_default()
}

pub struct Contributions;

impl Domain for Contributions {
    type Item = Contribution;
    const PAGE: Page = Page::Contributions;
    const PURPOSE: &'static str = "What is paid into each account, by whom, and how much";
    const PATH: &'static str = "contributions";
    const SINGULAR: &'static str = "Contribution";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::text("id", "ID")
            .help("A short handle scenarios address this contribution by. Optional."),
        FieldSpec::text("name", "Name")
            .help("What the contribution is called. Blank shows the ID."),
        FieldSpec::refers("to", "Pays into", RefSource::Account)
            .help("The account the money lands in."),
        FieldSpec::choice("by", "Paid by", Vocabulary::Payer)
            .help("You, out of your pay; an employer; or you, after tax, into a workplace plan."),
        FieldSpec::choice(AMOUNT_AS, "Amount as", Vocabulary::AmountForm)
            .derived(seed_amount_form, write_amount_form)
            .help("Dollars a year, a share of an income, the legal maximum, or a match."),
        FieldSpec::money(AMOUNT_KEY, "Amount")
            .shown_when(pays_dollars)
            .help("Paid in per year, in today's dollars."),
        FieldSpec::rate(RATE_KEY, "Rate")
            .shown_when(pays_share)
            .help("The share of the income's gross paid in each year, such as 6%."),
        FieldSpec::refers(OF_KEY, "Of", RefSource::Income)
            .shown_when(names_income)
            .help("The income the share is of. It needs an id to be named here."),
        FieldSpec::rate("step.add", "Steps up by")
            .shown_when(pays_share)
            .blank("Not at all")
            .help("Added to the rate each year after the first, such as 1%. Blank holds it."),
        FieldSpec::rate("step.up_to", "Up to")
            .shown_when(pays_share)
            .help("The rate the steps stop at."),
        FieldSpec::rate(MATCH_RATE_KEY, "Match rate")
            .shown_when(pays_match)
            .help("The share of what you pay in that the employer matches, such as 50%."),
        FieldSpec::rate(MATCH_UP_TO_KEY, "Matched up to")
            .shown_when(pays_match)
            .help("The share of the income's gross the match stops at, such as 6%."),
        FieldSpec::timing(),
        FieldSpec::starts(),
        FieldSpec::ends().help("The last year it is paid. Blank means the end of the plan."),
        FieldSpec::once().help("The one year it is paid."),
        FieldSpec::growth("cola", "Growth")
            .shown_when(pays_dollars)
            .help(GROWTH_HELP),
    ];
    const COLUMNS: &'static [Column] = &[
        Column::new("id").headed("Contribution"),
        Column::new("to").headed("Pays into"),
        Column::new("by").headed("Paid by"),
        Column::new(AMOUNT_KEY)
            .headed("Amount")
            .phrased(amount_phrase),
        Column::new("start").headed("From"),
        Column::new("end").headed("Until"),
    ];
    const BLANK: &'static str = r#"
to = ""
amount = 0
"#;

    fn items(plan: &Plan) -> &[Contribution] {
        &plan.contributions
    }

    fn items_mut(plan: &mut Plan) -> &mut Vec<Contribution> {
        &mut plan.contributions
    }
}
