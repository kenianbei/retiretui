use retiretui_engine::plan::{Cliff, Expense, Plan};
use toml::{Table, Value};

use super::applies::{self, ONCE};
use super::cells::Column;
use super::{Domain, DomainId, FieldSpec, GROWTH_HELP};
use crate::present;

const PHASES: &str = "phases";
const PHASE_FROM_HELP: &str = "When the amount changes. A date or an age splits that year by months; a blank phase is dropped.";
const PHASE_AMOUNT_HELP: &str = "Per year from then on, in today's dollars.";

fn phases(item: &Table) -> impl Iterator<Item = &Value> {
    item.get(PHASES)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn is_filled_phase(phase: &Value) -> bool {
    phase.as_table().is_some_and(|phase| !phase.is_empty())
}

/// Phase `AT` is offered while the expense recurs and it or a later phase
/// holds anything, and after the last that does: every filled phase, then
/// one blank one.
fn phase_shown<const AT: usize>(item: &Table) -> bool {
    applies::recurs(item) && (AT == 0 || phases(item).skip(AT - 1).any(is_filled_phase))
}

/// Drops the blank phases, and every phase of an expense made one-time.
fn write_phases(item: &mut Table, timing: &Value) {
    if timing.as_str() == Some(ONCE) {
        item.remove(PHASES);
        return;
    }
    if let Some(Value::Array(rows)) = item.get_mut(PHASES) {
        rows.retain(is_filled_phase);
        if rows.is_empty() {
            item.remove(PHASES);
        }
    }
}

/// How many phases the expense has, nothing where it has none.
fn phase_count(item: &Table, _: &Plan) -> String {
    match phases(item).count() {
        0 => String::new(),
        count => count.to_string(),
    }
}

const fn phase_from(
    key: &'static str,
    label: &'static str,
    shown: fn(&Table) -> bool,
) -> FieldSpec {
    FieldSpec::trigger(key, label)
        .shown_when(shown)
        .help(PHASE_FROM_HELP)
}

const fn phase_amount(key: &'static str, shown: fn(&Table) -> bool) -> FieldSpec {
    FieldSpec::money(key, "  Amount")
        .shown_when(shown)
        .help(PHASE_AMOUNT_HELP)
}

/// The plan's expenses, edited as a table.
pub struct Expenses;

impl Domain for Expenses {
    type Item = Expense;
    const ID: DomainId = DomainId::Expenses;
    const PURPOSE: &'static str = "What you spend each year, and when";
    const PATH: &'static str = "expenses";
    const SINGULAR: &'static str = "Expense";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::name("What the spending is for. Blank shows the ID."),
        FieldSpec::text("id", "ID")
            .help("A short unique handle scenarios refer to this expense by."),
        FieldSpec::money("amount", "Annual amount").help("Per year, in today's dollars."),
        FieldSpec::timing().derived(applies::seed_timing, write_phases),
        FieldSpec::starts(),
        FieldSpec::ends(),
        FieldSpec::once(),
        FieldSpec::growth("cola", "Growth").help(GROWTH_HELP),
        phase_from("phases.0.from", "Phase 2 from", phase_shown::<0>),
        phase_amount("phases.0.amount", phase_shown::<0>),
        phase_from("phases.1.from", "Phase 3 from", phase_shown::<1>),
        phase_amount("phases.1.amount", phase_shown::<1>),
        phase_from("phases.2.from", "Phase 4 from", phase_shown::<2>),
        phase_amount("phases.2.amount", phase_shown::<2>),
        phase_from("phases.3.from", "Phase 5 from", phase_shown::<3>),
        phase_amount("phases.3.amount", phase_shown::<3>),
        phase_from("phases.4.from", "Phase 6 from", phase_shown::<4>),
        phase_amount("phases.4.amount", phase_shown::<4>),
        phase_from("phases.5.from", "Phase 7 from", phase_shown::<5>),
        phase_amount("phases.5.amount", phase_shown::<5>),
        FieldSpec::flag("medical", "Medical").help(
            "Health spending an HSA can pay without tax. HSA withdrawals beyond it are taxed, and cost 20% more before 65.",
        ),
        FieldSpec::flag("essential", "Essential")
            .help("Spending you could not cut. The spending ceiling leaves it as it is."),
    ];
    const COLUMNS: &'static [Column] = &[
        Column::new("id").headed("Expense"),
        Column::new("amount").headed("Amount"),
        Column::new(PHASES).headed("Phases").phrased(phase_count),
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

/// The MAGI cliffs the plan declares, edited as a table.
pub struct Cliffs;

impl Domain for Cliffs {
    type Item = Cliff;
    const ID: DomainId = DomainId::Cliffs;
    const PURPOSE: &'static str = "Costs that start once your income passes a threshold";
    const PATH: &'static str = "cliffs";
    const SINGULAR: &'static str = "Cliff";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::name(
            "What crossing the threshold costs you, such as an ACA premium credit. Blank shows the ID.",
        ),
        FieldSpec::text("id", "ID").help("A short unique handle scenarios refer to this cliff by."),
        FieldSpec::money("magi_over", "MAGI threshold").help(
            "A year whose modified adjusted gross income exceeds this pays the cost that year.",
        ),
        FieldSpec::money("cost", "Annual cost")
            .help("What the year costs once the threshold is crossed."),
        FieldSpec::trigger("start", "Starts")
            .blank(present::PLAN_START)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn written(text: &str, timing: &str) -> Table {
        let mut item: Table = text.parse().unwrap();
        write_phases(&mut item, &Value::String(timing.to_owned()));
        item
    }

    const PHASED: &str =
        "amount = 1\nphases = [{ from = { age = 70, owner = \"me\" }, amount = 2 }, {}]";

    #[test]
    fn a_blank_phase_is_dropped_and_a_one_time_expense_keeps_none() {
        let kept = written(PHASED, applies::EVERY_YEAR);
        assert_eq!(phases(&kept).count(), 1, "{kept}");
        assert!(!written("amount = 1\nphases = [{}]", applies::EVERY_YEAR).contains_key(PHASES));
        assert!(!written(PHASED, ONCE).contains_key(PHASES));
    }

    #[test]
    fn the_next_phase_is_offered_after_the_last_filled_one() {
        let item: Table = PHASED.parse().unwrap();
        assert!(phase_shown::<0>(&item) && phase_shown::<1>(&item));
        assert!(!phase_shown::<2>(&item));
        let once: Table = format!("{PHASED}\nhappens = \"{ONCE}\"").parse().unwrap();
        assert!(!phase_shown::<0>(&once));
    }

    #[test]
    fn the_read_out_lists_the_filled_phases_alone() {
        let plan = Plan::from_toml_str(crate::overview::tests::TEST_PLAN).unwrap();
        let item: Table = "amount = 1\nphases = [{ from = { date = 2030-07-01 }, amount = 2 }]"
            .parse()
            .unwrap();
        let rows = super::super::details::rows(&super::super::Form::of::<Expenses>(), &item, &plan);
        let said: Vec<(String, String)> = rows
            .into_iter()
            .filter(|row| row.label.contains("Phase") || row.label.contains("Amount"))
            .map(|row| (row.label, row.text))
            .collect();
        assert_eq!(
            said,
            [
                ("Phase 2 from".to_owned(), "Jul 2030".to_owned()),
                ("  Amount".to_owned(), "$2".to_owned()),
            ],
            "the filled phase, not the one the form offers next",
        );
    }

    #[test]
    fn the_phases_column_counts_them() {
        let plan = Plan::from_toml_str(crate::overview::tests::TEST_PLAN).unwrap();
        assert_eq!(phase_count(&PHASED.parse().unwrap(), &plan), "2");
        assert_eq!(phase_count(&"amount = 1".parse().unwrap(), &plan), "");
    }
}
