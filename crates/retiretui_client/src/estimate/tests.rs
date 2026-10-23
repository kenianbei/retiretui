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

fn cells(estimator: &Estimator) -> Vec<[String; 5]> {
    let found = estimator.estimate();
    listed(found)
        .map(|row| row.cells(MoneyForm::Full))
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
            "  Unnamed payees",
            "    SOMEONE",
        ]
    );
    let found = estimator.estimate();
    let rows: Vec<Listed> = listed(found).collect();
    assert_eq!(rows[3].title(), "Category");
    let read = rows[3].read_out(found.months, MoneyForm::Full);
    assert_eq!(read[1].1, "Other (new bucket)");
    assert_eq!(
        read[2].1, "nothing else fits it",
        "living expenses names no food"
    );
    assert_eq!(
        summary(found, MoneyForm::Full),
        "1 month through Oct 2026: $7,080 spent, $0 of income, $0 in transfers a year; 1 payee unnamed, $360 a year."
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
    assert_eq!(
        applied,
        Applied::Changed(vec![Change::Name {
            payee: "SOMEONE".to_owned(),
            category: Category::Pets,
        }])
    );
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
        dining.read_out(found.months, MoneyForm::Full)[2].1,
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
fn a_change_is_made_to_the_users_patterns_as_they_wrote_them() {
    let named = Change::Name {
        payee: "AL'S \"PLACE\"".to_owned(),
        category: Category::Dining,
    };
    let file = "# mine\n[[patterns]]\nmatch = \"SAFEWAY\"\ncategory = \"groceries\"\n";
    let text = named.applied(file).unwrap();
    assert!(text.starts_with(file), "{text}");
    let written: Table = text.parse().unwrap();
    assert_eq!(
        written["patterns"][1]["is"].as_str(),
        Some("AL'S \"PLACE\""),
        "{text}"
    );

    let merge = |payee: &str| Change::Merge {
        payee: payee.to_owned(),
        name: "Acme".to_owned(),
    };
    let text = merge("ACME HELP").applied("").unwrap();
    let text = merge("ACME SHOP").applied(&text).unwrap();
    assert_eq!(text.matches("name = ").count(), 1, "{text}");
    let mut patterns = Patterns::default();
    patterns.add(&text).unwrap();
    assert_eq!(patterns.name_of("ACME SHOP"), "Acme");

    let inline = "patterns = [{ match = \"X\", category = \"pets\" }]\n";
    let text = named.applied(inline).unwrap();
    assert!(text.contains("{ is = "), "kept inline: {text}");
    assert!(named.applied("patterns = 3").is_err());
}

#[test]
fn an_ignore_the_page_wrote_is_taken_back_and_a_hand_written_one_is_not() {
    let record = Record {
        payee: "SOMEONE".to_owned(),
        on: retiretui_engine::plan::PlanDate(jiff::civil::date(2026, 10, 3)),
        spent: retiretui_engine::estimate::Cents(3010),
    };
    let ignored = Change::IgnoreRecord(record.clone()).applied("").unwrap();
    assert!(ignored.contains("spent = [30.1, 30.1]"), "{ignored}");
    let counted = Change::Unignore(record.clone()).applied(&ignored).unwrap();
    assert!(!counted.contains("ignore"), "{counted}");
    let payee = Change::IgnorePayee {
        payee: "someone".to_owned(),
    };
    let counted = Change::Unignore(record.clone())
        .applied(&payee.applied("").unwrap())
        .unwrap();
    assert!(!counted.contains("ignore"), "the payee's, else: {counted}");
    let by_hand = "[[patterns]]\nmatch = \"SOME\"\nignore = true\n";
    assert!(Change::Unignore(record).applied(by_hand).is_err());
}

#[test]
fn a_record_is_ignored_and_counted_again_on_the_ignored_row() {
    let plan = retired();
    let mut estimator = estimated(&plan);
    let dining = 3;
    let change = estimator
        .act(dining, Some(0), Action::IgnoreRecord)
        .unwrap();
    assert!(matches!(change, Change::IgnoreRecord(_)), "{change:?}");
    assert!(estimator.refresh(&plan));
    let found = estimator.estimate();
    assert_eq!((found.ignored, found.ignored_rows), (480, 1));
    let last = listed(found).last().unwrap();
    assert_eq!(last.cells(MoneyForm::Full)[0], "Ignored");
    assert!(summary(found, MoneyForm::Full).ends_with("; 1 record ignored, $480 a year."));
    let at = listed(found).count() - 1;
    assert_eq!(
        estimator.behind(at, MoneyForm::Full).unwrap().toggle,
        Some("Count")
    );
    assert!(matches!(
        estimator.act(at, Some(0), Action::IgnoreRecord).unwrap(),
        Change::Unignore(_)
    ));
    assert!(estimator.refresh(&plan));
    assert_eq!(estimator.estimate().ignored_rows, 0);
    assert!(
        estimator.act(0, Some(0), Action::IgnoreRecord).is_err(),
        "an expense's payees"
    );
}

#[test]
fn the_overlay_states_what_the_proposals_change() {
    let plan = retired();
    let estimator = estimated(&plan);
    let text = overlay("base.toml", &plan, estimator.estimate()).unwrap();
    assert!(text.contains("base = \"base.toml\""), "{text}");
    assert!(text.contains("id = \"other\""), "{text}");
}

#[test]
fn a_row_says_what_it_changes_why_and_what_lies_behind_it() {
    let estimator = estimated(&retired());
    let rows = cells(&estimator);
    let now = estimator.estimate().proposals[0].now.unwrap();
    assert_eq!(
        rows[0][1..4],
        [
            MoneyForm::Full.money(now),
            "$6,000".to_owned(),
            MoneyForm::Full.signed(6000 - now)
        ]
    );
    assert_eq!(rows[2][1..4], ["-", "$1,080", "+$1,080"], "a new bucket");
    assert_eq!(rows[3][4], "bucket");
    let payees = estimator.behind(0, MoneyForm::Full).unwrap();
    assert_eq!(payees.title, "Payees: Housing");
    assert_eq!(payees.rows, [["ROCKET MORTGAGE", "$6,000"]]);
    let dining = estimator.behind(3, MoneyForm::Full).unwrap();
    assert_eq!(dining.title, "Records: Dining");
    assert_eq!(dining.rows, [["2026-10-04", "TAQUERIA SOL", "$40.00"]]);
    let unnamed = estimator.behind(5, MoneyForm::Full).unwrap();
    assert_eq!(unnamed.rows, [["2026-10-03", "SOMEONE", "$30.00"]]);
}

#[test]
fn a_payee_is_moved_named_or_ignored_from_its_record_and_alike_ones_by_words() {
    let plan = retired();
    let mut estimator = estimated(&plan);
    let dining = 3;
    let moved = estimator
        .act(dining, Some(0), Action::PayeeIs(Category::Travel))
        .unwrap();
    assert_eq!(
        moved,
        Change::Name {
            payee: "TAQUERIA SOL".to_owned(),
            category: Category::Travel,
        }
    );
    assert!(estimator.refresh(&plan));
    let reach = estimator.reach("taqueria");
    assert_eq!((reach.payees, reach.spent), (1, 480));
    assert_eq!(estimator.reach("  ").payees, 0);

    let someone = listed(estimator.estimate())
        .position(|row| matches!(row, Listed::Payee(_)))
        .unwrap();
    let ignored = estimator.act(someone, None, Action::IgnorePayee).unwrap();
    assert_eq!(ignored.said(), "SOMEONE is ignored");
    assert!(estimator.refresh(&plan));
    assert_eq!(estimator.estimate().ignored_rows, 1);

    let contains = estimator
        .act(
            0,
            None,
            Action::Containing(" rocket ".to_owned(), Category::Housing),
        )
        .unwrap();
    assert_eq!(contains.said(), "payees containing rocket are Housing");
    assert!(
        estimator
            .act(0, None, Action::Containing(" ".to_owned(), Category::Pets))
            .is_err()
    );
    assert!(
        estimator.act(0, None, Action::IgnorePayee).is_err(),
        "an expense has no payee"
    );
}
