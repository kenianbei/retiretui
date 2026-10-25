use std::path::PathBuf;

use bevy_app::App;
use plurimus::term::KeyCode;

use retiretui_client::estimate::RowForm;

use super::edits::NO_EXPORTS;
use super::*;
use crate::compare::Compared;
use crate::support::{
    Headless, SETTLING_TICKS, SIZE, answer_back, composed_frame, headless_app_at, is_asking,
    is_browsing, picker_rows, press_key, said, scratch_workspace, show, type_text,
};

const CHASE: &str = include_str!("../../../../retiretui_engine/tests/fixtures/bank/chase.csv");
const OFX: &str = include_str!("../../../../retiretui_engine/tests/fixtures/bank/checking.ofx");

/// A workspace holding the test plan, opened on the Spending Estimate
/// page, with `chase.csv` beside it.
fn workspace_app() -> (PathBuf, Headless) {
    let dir = scratch_workspace(crate::support::TEST_PLAN);
    std::fs::write(dir.join("chase.csv"), CHASE).unwrap();
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::SpendingEstimate);
    (dir, app)
}

fn add(app: &mut App, name: &str, text: &str) {
    app.world_mut()
        .resource_mut::<Held>()
        .0
        .add(name, text)
        .unwrap();
    for _ in 0..SETTLING_TICKS {
        app.update();
    }
}

fn expense_ids(app: &App) -> Vec<String> {
    let stated = app.world().resource::<Draft>().plan.expenses.iter();
    stated.map(|it| it.id.clone()).collect()
}

#[test]
fn the_page_says_what_to_add_until_an_export_is_read() {
    let (_, mut app) = workspace_app();
    let frame = composed_frame(&app);
    assert!(frame.contains("╭ Files "), "{frame}");
    assert!(frame.contains("╭ Proposals "), "{frame}");
    assert!(
        frame.contains("Add a bank's CSV, OFX or QFX export"),
        "{frame}"
    );
    assert!(
        frame.contains("f add file") && !frame.contains("t take"),
        "{frame}"
    );
    press_key(&mut app, KeyCode::Char('t'));
    assert!(!is_asking(&app));
    add(&mut app, "chase.csv", CHASE);
    add(&mut app, "checking.ofx", OFX);
    let frame = composed_frame(&app);
    assert!(frame.contains("chase.csv  5 rows"), "{frame}");
    assert!(frame.contains("checking.ofx  2 rows"), "{frame}");
    assert!(frame.contains("Groceries"), "{frame}");
    assert!(frame.contains("Utilities"), "{frame}");
    assert!(frame.contains("Groceries (new)"), "{frame}");
    assert!(frame.contains("╭ Settings "), "{frame}");
    assert!(frame.contains("Large share  10%"), "{frame}");
    assert!(frame.contains("SAFEWAY"), "{frame}");
    assert!(frame.contains(ON_PROPOSALS), "{frame}");
    assert!(
        frame.contains("t take") && frame.contains("w write"),
        "{frame}"
    );
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Tab);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("CITY OF SAN JOSE WATER"), "{frame}");
    assert!(frame.contains("╭ Payees: Utilities "), "{frame}");
    assert!(frame.contains("1 month through Oct 2026"), "{frame}");
    app.world_mut().insert_resource(Cursor(3));
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("╭ Records: Utilities "), "{frame}");
    assert!(frame.contains("2026-10-"), "a record's date: {frame}");
}

#[test]
fn an_export_is_picked_by_its_extension_and_dropped_from_the_files() {
    let (_, mut app) = workspace_app();
    press_key(&mut app, KeyCode::Char('f'));
    assert!(is_browsing(&app));
    let names: Vec<String> = picker_rows(&mut app)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert!(
        names.iter().any(|name| name.contains("chase.csv")),
        "{names:?}"
    );
    assert!(
        !names.iter().any(|name| name.contains("plan.toml")),
        "{names:?}"
    );
    type_text(&mut app, "chase.csv");
    press_key(&mut app, KeyCode::Enter);
    for _ in 0..4 {
        app.update();
    }
    assert!(
        said(&app)
            .iter()
            .any(|line| line.contains("read 5 rows from chase.csv")),
        "{:?}",
        said(&app)
    );
    assert!(composed_frame(&app).contains("Groceries"));
    press_key(&mut app, KeyCode::Char('x'));
    app.update();
    app.update();
    let frame = composed_frame(&app);
    assert!(!frame.contains("chase.csv  5 rows"), "{frame}");
    assert!(frame.contains(NO_FILES), "{frame}");
    assert!(
        frame.contains("Add a bank's CSV, OFX or QFX export"),
        "{frame}"
    );
    press_key(&mut app, KeyCode::Char('x'));
    assert!(
        said(&app).iter().any(|line| line.contains(NO_EXPORTS)),
        "{:?}",
        said(&app)
    );
}

#[test]
fn taking_the_proposals_sets_and_adds_the_expenses_after_asking() {
    let (_, mut app) = workspace_app();
    add(&mut app, "chase.csv", CHASE);
    let before = expense_ids(&app);
    press_key(&mut app, KeyCode::Char('t'));
    assert!(is_asking(&app));
    answer_back(&mut app, 0);
    app.update();
    let after = expense_ids(&app);
    assert_eq!(after.len(), before.len() + 1, "{after:?}");
    assert_eq!(after.last().map(String::as_str), Some("groceries"));
    assert!(app.world().resource::<Draft>().is_dirty());
    assert!(
        said(&app).iter().any(|line| line.contains("1 added")),
        "{:?}",
        said(&app)
    );
}

#[test]
fn the_highlighted_row_opens_its_form_and_the_edit_lasts() {
    let (_, mut app) = workspace_app();
    add(&mut app, "chase.csv", CHASE);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let frame = composed_frame(&app);
    assert!(
        frame.contains("Expense") && frame.contains("Amount"),
        "{frame}"
    );
    assert!(frame.contains("Groceries"), "{frame}");
    press_key(&mut app, KeyCode::Esc);
    app.update();
    let proposed = app.world().resource::<Held>().0.estimate().proposals[0].amount;
    assert!(proposed > 0);
}

#[test]
fn writing_the_proposals_makes_a_scenario_over_the_document() {
    let (dir, mut app) = workspace_app();
    add(&mut app, "chase.csv", CHASE);
    press_key(&mut app, KeyCode::Char('w'));
    assert!(is_browsing(&app));
    type_text(&mut app, "estimated.toml");
    press_key(&mut app, KeyCode::Enter);
    for _ in 0..4 {
        app.update();
    }
    let written = std::fs::read_to_string(dir.join("estimated.toml")).unwrap();
    assert!(written.contains("base = \"plan.toml\""), "{written}");
    assert!(written.contains("id = \"groceries\""), "{written}");
    assert!(
        app.world()
            .resource::<Compared>()
            .paths()
            .any(|path| path.ends_with("estimated.toml"))
    );
}

/// Groceries, and a payee nothing names.
const NAMELESS: &str =
    "Date,Description,Amount\n2026-10-01,SAFEWAY,-300.00\n2026-10-02,SOMEONE,-30.00\n";

/// Puts the cursor on the row at `at` and applies `answers` to it as its
/// form, `row_form`, would.
fn apply_at(app: &mut App, at: usize, row_form: RowForm, answers: toml::Table) {
    app.world_mut().resource_mut::<Cursor>().0 = at;
    let mut draft = app.world_mut().resource_mut::<Draft>();
    (row_form.form().store)(&mut draft, 0, answers).unwrap();
    app.world_mut().run_system_cached(edits::apply_row).unwrap();
    app.update();
}

fn answer(key: &str, value: &str) -> toml::Table {
    let mut table = toml::Table::new();
    table.insert(key.to_owned(), toml::Value::String(value.to_owned()));
    table
}

#[test]
fn a_category_moved_is_kept_in_the_plan_as_one_step() {
    let (_, mut app) = workspace_app();
    add(&mut app, "nameless.csv", NAMELESS);
    let rows: Vec<String> = {
        let found = app.world().resource::<Held>().0.estimate().clone();
        listed(&found)
            .map(|row| row.cells(MoneyForm::Full)[0].clone())
            .collect()
    };
    let at = rows.iter().position(|row| row == "  Groceries").unwrap();
    apply_at(&mut app, at, RowForm::Move, answer("expense", "living"));
    let draft = app.world().resource::<Draft>();
    let assigned = &draft.plan.estimate.as_ref().unwrap().assign;
    assert_eq!(assigned[0].expense, "living");
    assert!(draft.is_dirty() && draft.can_undo());
    let found = app.world().resource::<Held>().0.estimate();
    assert_eq!(
        found.proposals[0].id, "living",
        "estimated again over the plan"
    );
}

#[test]
fn a_named_payee_is_remembered_in_the_patterns_file_the_launch_names() {
    let (dir, mut app) = workspace_app();
    let file = dir.join("config").join("patterns.toml");
    app.world_mut()
        .insert_resource(PatternsFile(Some(file.clone())));
    add(&mut app, "nameless.csv", NAMELESS);
    let at = {
        let found = app.world().resource::<Held>().0.estimate().clone();
        listed(&found)
            .position(|row| matches!(row, retiretui_client::estimate::Listed::Payee(_)))
            .unwrap()
    };
    apply_at(&mut app, at, RowForm::Payee, answer("category", "pets"));
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("is = \"SOMEONE\""), "{written}");
    assert!(
        said(&app)
            .iter()
            .any(|line| line.contains("SOMEONE is Pets, kept in"))
    );
    assert!(
        app.world()
            .resource::<Held>()
            .0
            .estimate()
            .unknown
            .is_empty()
    );
}

/// Puts the cursor on the row at `at` and the keyboard on the Records
/// pane, then types `keys`.
fn on_record(app: &mut App, at: usize, keys: &[KeyCode]) {
    app.world_mut().insert_resource(Cursor(at));
    for _ in 0..SETTLING_TICKS {
        app.update();
    }
    let records = app
        .world_mut()
        .query_filtered::<bevy_ecs::prelude::Entity, With<BehindTable>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .insert_resource(InputFocus::from_entity(records));
    for &key in keys {
        press_key(app, key);
        for _ in 0..SETTLING_TICKS {
            app.update();
        }
    }
}

#[test]
fn a_record_is_ignored_from_the_records_pane_and_counted_again() {
    let (dir, mut app) = workspace_app();
    let file = dir.join("config").join("patterns.toml");
    app.world_mut()
        .insert_resource(PatternsFile(Some(file.clone())));
    add(&mut app, "chase.csv", CHASE);
    let groceries = {
        let found = app.world().resource::<Held>().0.estimate().clone();
        listed(&found)
            .position(|row| matches!(row, retiretui_client::estimate::Listed::Category { .. }))
            .unwrap()
    };
    on_record(&mut app, groceries, &[KeyCode::Char('i')]);
    let found = app.world().resource::<Held>().0.estimate().clone();
    assert_eq!(found.ignored_rows, 1);
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("ignore = true")
    );
    assert!(composed_frame(&app).contains("Ignored"));

    on_record(&mut app, listed(&found).count() - 1, &[KeyCode::Char('i')]);
    assert_eq!(app.world().resource::<Held>().0.estimate().ignored_rows, 0);
    assert!(!std::fs::read_to_string(&file).unwrap().contains("ignore"));
}

fn is_picking(app: &App) -> bool {
    app.world().resource::<crate::picker::Picking>().is_open()
}

/// The first category row of the estimate.
fn groceries_row(app: &App) -> usize {
    let found = app.world().resource::<Held>().0.estimate().clone();
    listed(&found)
        .position(|row| matches!(row, retiretui_client::estimate::Listed::Category { .. }))
        .unwrap()
}

#[test]
fn a_payee_is_moved_by_its_category_number_or_its_words_from_the_menu() {
    let (dir, mut app) = workspace_app();
    let file = dir.join("config").join("patterns.toml");
    app.world_mut()
        .insert_resource(PatternsFile(Some(file.clone())));
    add(&mut app, "chase.csv", CHASE);
    let at = groceries_row(&app);
    on_record(&mut app, at, &[KeyCode::Char('c'), KeyCode::Char('3')]);
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(
        written.contains("category = \"utilities\""),
        "3 is at once: {written}"
    );
    assert!(said(&app).iter().any(|line| line.contains("is Utilities")));

    let at = groceries_row(&app);
    on_record(&mut app, at, &[KeyCode::Char('c'), KeyCode::Char('1')]);
    assert!(is_picking(&app), "1 could still be 10 to 19");
    type_text(&mut app, "2");
    for _ in 0..SETTLING_TICKS {
        app.update();
    }
    assert!(!is_picking(&app), "12 is one number");
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("category = \"internet\""), "{written}");

    let at = groceries_row(&app);
    on_record(
        &mut app,
        at,
        &[KeyCode::Char('c'), KeyCode::Char('1'), KeyCode::Enter],
    );
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(
        written.contains("category = \"groceries\""),
        "1 then ⏎: {written}"
    );
}

#[test]
fn c_0_sends_a_payee_to_the_bucket() {
    let (dir, mut app) = workspace_app();
    let file = dir.join("config").join("patterns.toml");
    app.world_mut()
        .insert_resource(PatternsFile(Some(file.clone())));
    add(&mut app, "chase.csv", CHASE);
    let at = groceries_row(&app);
    on_record(&mut app, at, &[KeyCode::Char('c'), KeyCode::Char('0')]);
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("bucket = true"), "0 is at once: {written}");
    assert!(
        said(&app)
            .iter()
            .any(|line| line.contains("goes to the bucket"))
    );
}

#[test]
fn payees_containing_a_records_words_are_moved_from_its_menu() {
    let (dir, mut app) = workspace_app();
    let file = dir.join("config").join("patterns.toml");
    app.world_mut()
        .insert_resource(PatternsFile(Some(file.clone())));
    add(&mut app, "chase.csv", CHASE);
    let at = groceries_row(&app);
    on_record(&mut app, at, &[KeyCode::Enter]);
    assert!(composed_frame(&app).contains("Payees containing…"));
    type_text(&mut app, "payees containing");
    press_key(&mut app, KeyCode::Enter);
    for _ in 0..SETTLING_TICKS {
        app.update();
    }
    let frame = composed_frame(&app);
    assert!(frame.contains("1 payee, $1,706 a year"), "{frame}");
    press_key(&mut app, KeyCode::Enter);
    for _ in 0..SETTLING_TICKS {
        app.update();
    }
    type_text(&mut app, "18");
    for _ in 0..SETTLING_TICKS {
        app.update();
    }
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("match = "), "{written}");
    assert!(written.contains("category = \"pets\""), "{written}");
}

/// The place of the first payee nothing names among the held rows.
fn first_payee(app: &App) -> usize {
    let found = app.world().resource::<Held>().0.estimate().clone();
    listed(&found)
        .position(|row| matches!(row, retiretui_client::estimate::Listed::Payee(_)))
        .unwrap()
}

#[test]
fn a_patterns_file_that_cannot_be_read_is_left_as_it_was() {
    let (dir, mut app) = workspace_app();
    let file = dir.join("patterns.toml");
    let unreadable = b"patterns = []\n# \xff\n";
    std::fs::write(&file, unreadable).unwrap();
    app.world_mut()
        .insert_resource(PatternsFile(Some(file.clone())));
    add(&mut app, "nameless.csv", NAMELESS);
    let at = first_payee(&app);
    apply_at(&mut app, at, RowForm::Payee, answer("category", "pets"));
    assert_eq!(std::fs::read(&file).unwrap(), unreadable);
    assert!(
        said(&app)
            .iter()
            .any(|line| line.contains("SOMEONE is Pets, but not kept")),
        "{:?}",
        said(&app)
    );
}

#[test]
fn the_users_patterns_are_read_at_launch_and_one_that_does_not_parse_is_left_out() {
    let (dir, mut app) = workspace_app();
    let file = dir.join("patterns.toml");
    app.world_mut()
        .insert_resource(PatternsFile(Some(file.clone())));
    std::fs::write(
        &file,
        "patterns = [{ is = \"SOMEONE\", catgory = \"pets\" }]\n",
    )
    .unwrap();
    app.world_mut()
        .run_system_cached(edits::read_user_patterns)
        .unwrap();
    app.update();
    assert!(
        said(&app).iter().any(|line| line.contains("is left out")),
        "{:?}",
        said(&app)
    );
    std::fs::write(
        &file,
        "patterns = [{ is = \"SOMEONE\", category = \"pets\" }]\n",
    )
    .unwrap();
    app.world_mut()
        .run_system_cached(edits::read_user_patterns)
        .unwrap();
    add(&mut app, "nameless.csv", NAMELESS);
    let held = app.world().resource::<Held>();
    assert!(held.0.estimate().unknown.is_empty());
}
