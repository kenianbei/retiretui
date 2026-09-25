use retiretui_engine::plan::{Cliff, Expense, Plan};

use super::cells::Column;
use super::domain::{Domain, FieldSpec, GROWTH_HELP};
use crate::commands::tui::nav::Page;

pub struct Expenses;

impl Domain for Expenses {
    type Item = Expense;
    const PAGE: Page = Page::Expenses;
    const PURPOSE: &'static str = "What you spend each year, and when";
    const PATH: &'static str = "expenses";
    const SINGULAR: &'static str = "Expense";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::text("id", "ID")
            .help("A short unique handle scenarios refer to this expense by."),
        FieldSpec::text("name", "Name").help("What the spending is for. Blank shows the ID."),
        FieldSpec::money("amount", "Annual amount").help("Per year, in today's dollars."),
        FieldSpec::timing(),
        FieldSpec::starts(),
        FieldSpec::ends(),
        FieldSpec::once(),
        FieldSpec::growth("cola", "Growth").help(GROWTH_HELP),
    ];
    const COLUMNS: &'static [Column] = &[
        Column::new("id").headed("Expense"),
        Column::new("amount").headed("Amount"),
        Column::new("start").headed("From"),
        Column::new("end").headed("Until"),
        Column::new("cola"),
    ];
    const BLANK: &'static str = "
amount = 0
";

    fn items(plan: &Plan) -> &[Expense] {
        &plan.expenses
    }

    fn items_mut(plan: &mut Plan) -> &mut Vec<Expense> {
        &mut plan.expenses
    }
}

pub struct Cliffs;

impl Domain for Cliffs {
    type Item = Cliff;
    const PAGE: Page = Page::Cliffs;
    const PURPOSE: &'static str = "Costs that start once your income passes a threshold";
    const PATH: &'static str = "cliffs";
    const SINGULAR: &'static str = "Cliff";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::text("id", "ID")
            .help("A short unique handle scenarios refer to this cliff by."),
        FieldSpec::text("name", "Name").help(
            "What crossing the threshold costs you, such as an ACA premium credit. Blank shows the ID.",
        ),
        FieldSpec::money("magi_over", "MAGI threshold").help(
            "A year whose modified adjusted gross income exceeds this pays the cost that year.",
        ),
        FieldSpec::money("cost", "Annual cost")
            .help("What the year costs once the threshold is crossed."),
        FieldSpec::trigger("start", "Starts")
            .blank("Plan start")
            .help("When it begins. Blank means the start of the plan."),
        FieldSpec::trigger("end", "Ends").blank("Age 65").help(
            "The last year it applies. Blank means the year before the youngest person turns 65.",
        ),
        FieldSpec::growth("cola", "Growth")
            .help("For the threshold and the cost alike: Inflation, Fixed, or a rate such as 3%."),
    ];
    const COLUMNS: &'static [Column] = &[
        Column::new("id").headed("Cliff"),
        Column::new("magi_over"),
        Column::new("cost"),
        Column::new("start").headed("From"),
        Column::new("end").headed("Until"),
        Column::new("cola"),
    ];
    const BLANK: &'static str = "
magi_over = 0
cost = 0
";

    fn items(plan: &Plan) -> &[Cliff] {
        &plan.cliffs
    }

    fn items_mut(plan: &mut Plan) -> &mut Vec<Cliff> {
        &mut plan.cliffs
    }
}
