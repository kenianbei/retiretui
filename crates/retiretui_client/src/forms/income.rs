use retiretui_engine::plan::{ColaAnchor, Income, Plan};
use toml::{Table, Value};

use super::cells::Column;
use super::offers::{RefSource, Vocabulary};
use super::{Domain, DomainId, FieldSpec, GROWTH_HELP, applies};
use crate::codec::set_path;
use crate::present;

/// The key of a salary's tick for paying into Social Security, which is
/// on by default: read from the settled value, written only when off.
const SOCIAL_SECURITY: &str = "social_security";

fn seed_social_security(item: &Table) -> Value {
    let stated = item.get(SOCIAL_SECURITY).and_then(Value::as_bool);
    Value::Boolean(stated.unwrap_or(true))
}

fn write_social_security(item: &mut Table, pays: &Value) {
    let differs = pays.as_bool().filter(|&pays| !pays);
    set_path(item, SOCIAL_SECURITY, differs.map(Value::Boolean));
}

/// The plan's incomes, edited as a table.
pub struct Incomes;

impl Domain for Incomes {
    type Item = Income;
    const ID: DomainId = DomainId::Income;
    const PURPOSE: &'static str = "Money coming in: salary, pensions, Social Security";
    const PATH: &'static str = "income";
    const SINGULAR: &'static str = "Income Source";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::name("What the income is called. Blank shows the ID."),
        FieldSpec::text("id", "ID")
            .blank("None")
            .help("A short handle, needed only when something else refers to this income."),
        FieldSpec::choice("kind", "Type", Vocabulary::IncomeKind)
            .help("The kind of income, which decides how it is taxed."),
        FieldSpec::refers("owner", "Owner", RefSource::Person).help("Who receives it."),
        FieldSpec::flag("covered", "Workplace plan")
            .shown_when(applies::can_be_covered)
            .help("A workplace plan covers its owner, contributing or not - a pension, a solo 401(k). Decides whether a traditional IRA contribution is deductible."),
        FieldSpec::money("pretax", "Before tax")
            .blank("None")
            .shown_when(applies::is_salary)
            .help("What the employer takes out before tax each year - health premiums, an FSA - in the amount's dollars. Reaches neither cash nor any tax."),
        FieldSpec::flag(SOCIAL_SECURITY, "Pays into SS")
            .shown_when(applies::is_salary)
            .derived(seed_social_security, write_social_security)
            .help("Off for a non-covered public job: no Social Security tax, Medicare's still, and no covered earnings recorded."),
        FieldSpec::money("amount", "Annual amount")
            .help("Per year, in today's dollars unless it grows from its first year. Blank on Social Security computes it from the earnings record."),
        FieldSpec::timing(),
        FieldSpec::starts(),
        FieldSpec::ends(),
        FieldSpec::once(),
        FieldSpec::growth("cola", "Growth")
            .help(GROWTH_HELP),
        FieldSpec::choice("cola_from", "Grows from", Vocabulary::ColaAnchor)
            .blank(present::cola_anchor(ColaAnchor::Plan))
            .shown_when(applies::can_grow_from_its_start)
            .help("Its first year takes the amount as what it pays that year, as a pension estimate states it."),
    ];
    const COLUMNS: &'static [Column] = &[
        Column::new("id").headed("Income"),
        Column::new("kind"),
        Column::new("owner"),
        Column::new("amount").headed("Amount"),
    ];
    const BLANK: &'static str = r#"
kind = "other"
owner = "{owner}"
amount = 0
"#;

    fn items(plan: &Plan) -> &[Income] {
        &plan.income
    }

    fn items_mut(plan: &mut Plan) -> &mut Vec<Income> {
        &mut plan.income
    }
}
