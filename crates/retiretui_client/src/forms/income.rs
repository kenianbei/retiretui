use retiretui_engine::plan::{Income, Plan};

use super::cells::Column;
use super::offers::{RefSource, Vocabulary};
use super::{Domain, DomainId, FieldSpec, GROWTH_HELP, applies};

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
            .shown_when(applies::is_salary)
            .help("A workplace plan at this job covers its owner, contributing or not - a pension plan, say. Decides whether a traditional IRA contribution is deductible."),
        FieldSpec::money("amount", "Annual amount")
            .help("Per year, in today's dollars. Blank on Social Security computes it from the earnings record."),
        FieldSpec::timing(),
        FieldSpec::starts(),
        FieldSpec::ends(),
        FieldSpec::once(),
        FieldSpec::growth("cola", "Growth")
            .help(GROWTH_HELP),
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
