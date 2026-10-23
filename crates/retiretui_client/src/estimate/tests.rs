use retiretui_engine::estimate::{Category, Patterns, Phases};
use retiretui_engine::plan::{Lean, Plan};
use toml::{Table, Value};

use super::*;
use crate::present::MoneyForm;
use crate::setup::examples::named;

/// A mortgage, a streaming service, a restaurant and a payee nothing names.
const CSV: &str = "Date,Description,Amount\n2026-10-01,ROCKET MORTGAGE,-500.00\n2026-10-02,NETFLIX,-20.00\n2026-10-03,SOMEONE,-30.00\n2026-10-04,TAQUERIA SOL,-40.00\n";

/// The retired couple's plan: housing, living, medicare, travel and care.
fn retired() -> Plan {
    let (_, _, text) = named("retired-couple.toml").expect("the example");
    Plan::from_toml_str(text).expect("the example parses")
}

fn cells(estimator: &Estimator) -> Vec<[String; 3]> {
    let found = estimator.estimate();
    listed(found)
        .map(|row| row.cells(found.months, MoneyForm::Full))
        .collect()
}

/// Holds `answers` as `form`'s, as opening it does.
fn hold(draft: &mut Draft, form: RowForm, answers: Table) {
    (form.form().store)(draft, 0, answers).unwrap();
}

fn estimated(plan: &Plan) -> Estimator {
    let mut estimator = Estimator::new(Patterns::embedded());
    estimator.add("x.csv", CSV).unwrap();
    assert!(estimator.refresh(plan));
    estimator
}

#[test]
fn each_expense_is_a_row_with_what_it_gathers_beneath_it() {
    let plan = retired();
    let estimator = estimated(&plan);
    assert!(!estimator.clone().refresh(&plan), "nothing moved");
    let names: Vec<String> = cells(&estimator)
        .into_iter()
        .map(|[name, ..]| name)
        .collect();
    assert_eq!(
        names,
        [
            "Housing",
            "  Housing",
            "Other (new bucket)",
            "  Dining",
            "  Subscriptions",
            "  payees nothing names",
            "SOMEONE",
        ]
    );
    let found = estimator.estimate();
    let rows: Vec<Listed> = listed(found).collect();
    assert_eq!(rows[3].title(), "Category");
    let read = rows[3].read_out(&plan, found.months, MoneyForm::Full);
    assert_eq!(read[1].1, "Other (new bucket)");
    assert_eq!(
        read[2].1, "nothing else fits it",
        "living expenses names no food"
    );
    assert_eq!(
        summary(found, MoneyForm::Full),
        "1 month through Oct 2026: $0 of income, $0 in transfers a year; 1 payee unnamed."
    );
    assert_eq!(
        take_question(found),
        "Set 1 expense and add 1 from 1 month of spending?"
    );
    assert!(
        estimator.form(5).is_none(),
        "the bucket's unnamed line edits nothing"
    );
}

#[test]
fn an_expense_a_category_and_a_payee_each_open_their_own_form() {
    let plan = retired();
    let mut draft = Draft::new(plan.clone(), false);
    let mut estimator = estimated(&plan);
    let (form, mut table) = estimator.form(0).unwrap();
    assert_eq!(form, RowForm::Set);
    table.insert("amount".to_owned(), Value::Integer(7000));
    table.insert("phases".to_owned(), Value::String("all".to_owned()));
    hold(&mut draft, RowForm::Set, table);
    assert_eq!(estimator.apply(0, &draft), Ok(Applied::Held));
    let housing = &estimator.estimate().proposals[0];
    assert_eq!((housing.amount, housing.phases), (7000, Phases::All));

    let (form, mut table) = estimator.form(2).unwrap();
    assert_eq!(form, RowForm::New);
    table.insert("name".to_owned(), Value::String("Odds and ends".to_owned()));
    hold(&mut draft, RowForm::New, table);
    estimator.apply(2, &draft).unwrap();
    assert_eq!(estimator.estimate().proposals[1].name, "Odds and ends");

    let (form, _) = estimator.form(3).unwrap();
    assert_eq!(form, RowForm::Move);
    let mut moved = Table::new();
    moved.insert("expense".to_owned(), Value::String("travel".to_owned()));
    hold(&mut draft, RowForm::Move, moved);
    let applied = estimator.apply(3, &draft).unwrap();
    assert_eq!(
        applied,
        Applied::Moved {
            category: Category::Dining,
            expense: Some("travel".to_owned()),
        }
    );

    let (form, table) = estimator.form(6).unwrap();
    assert_eq!((form, table), (RowForm::Payee, Table::new()));
    let mut named = Table::new();
    named.insert("category".to_owned(), Value::String("pets".to_owned()));
    hold(&mut draft, RowForm::Payee, named);
    let applied = estimator.apply(6, &draft).unwrap();
    assert!(matches!(
        applied,
        Applied::Named {
            category: Category::Pets,
            ..
        }
    ));
    assert!(estimator.refresh(&plan), "a named payee estimates again");
    assert!(estimator.estimate().unknown.is_empty());
}

#[test]
fn a_move_and_the_settings_are_kept_in_the_plan() {
    let mut plan = retired();
    assign(&mut plan, Category::Dining, Some("travel".to_owned()));
    let estimator = estimated(&plan);
    let names: Vec<String> = cells(&estimator)
        .into_iter()
        .map(|[name, ..]| name)
        .collect();
    assert!(names.iter().any(|name| name == "  Dining"));
    let found = estimator.estimate();
    let dining = listed(found)
        .find(|row| {
            matches!(row, Listed::Category { gathered, .. } if gathered.category == Category::Dining)
        })
        .unwrap();
    assert_eq!(
        dining.read_out(&plan, found.months, MoneyForm::Full)[2].1,
        "assigned in the plan"
    );
    assign(&mut plan, Category::Dining, None);
    assert_eq!(plan.estimate, None, "an empty table is dropped");

    let mut draft = Draft::new(plan, false);
    let mut table = (SETTINGS_FORM.item)(&draft, 0).unwrap();
    table.insert("lean".to_owned(), Value::String("existing".to_owned()));
    table.insert("bucket".to_owned(), Value::String("travel".to_owned()));
    (SETTINGS_FORM.store)(&mut draft, 0, table).unwrap();
    let settings = draft.plan.estimate.clone().unwrap();
    assert_eq!(
        (settings.lean, settings.bucket.as_deref()),
        (Lean::Existing, Some("travel"))
    );
    assert!(
        draft.plan.validate().is_empty(),
        "{:?}",
        draft.plan.validate()
    );
    let item = (SETTINGS_FORM.item)(&draft, 0).unwrap();
    let read = crate::forms::details::rows(&SETTINGS_FORM, &item, &draft.plan);
    assert_eq!(read[0].text, "Existing expenses only");
    assert_eq!(
        read.len(),
        2,
        "the large share is shown only while it is used"
    );
}

#[test]
fn a_payee_is_appended_to_the_users_patterns_and_still_reads() {
    let first = with_payee(None, "SOMEONE", Category::Pets).unwrap();
    let mut patterns = Patterns::default();
    patterns.add(&first).unwrap();
    assert_eq!(patterns.category("SOMEONE"), Some(Category::Pets));
    let second = with_payee(Some(&first), "AL'S \"PLACE\"", Category::Dining).unwrap();
    assert!(second.starts_with(&first));
    assert!(with_payee(Some("patterns = ["), "X", Category::Pets).is_err());
}

#[test]
fn the_overlay_states_what_the_proposals_change() {
    let plan = retired();
    let estimator = estimated(&plan);
    let text = overlay("base.toml", &plan, estimator.estimate()).unwrap();
    assert!(text.contains("base = \"base.toml\""), "{text}");
    assert!(text.contains("id = \"other\""), "{text}");
}
