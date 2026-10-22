use std::path::PathBuf;

use bevy_app::App;
use plurimus::term::KeyCode;

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
    for _ in 0..2 * SETTLING_TICKS {
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
    assert!(frame.contains("New expense"), "{frame}");
    assert!(frame.contains("SAFEWAY"), "{frame}");
    assert!(frame.contains(ON_PROPOSALS), "{frame}");
    assert!(
        frame.contains("t take") && frame.contains("w write"),
        "{frame}"
    );
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Tab);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("CITY OF SAN JOSE WATER"), "{frame}");
    assert!(frame.contains("1 month through Oct 2026"), "{frame}");
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
    let proposed = app.world().resource::<Estimated>().0.proposals[0].amount;
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
