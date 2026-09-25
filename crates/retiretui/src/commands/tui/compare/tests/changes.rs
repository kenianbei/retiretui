use plurimus::ui::{ComputedWidgetArea, UiLabel};

use super::super::changes::{THE_BASELINE, THE_SAME};
use super::*;

/// The Changes pane's rows, as they read.
fn rows(app: &mut App) -> Vec<String> {
    let list = single::<ChangesList>(app);
    let children = app.world().get::<Children>(list).unwrap().to_vec();
    children
        .iter()
        .map(|&row| app.world().get::<UiLabel>(row).unwrap().0.to_string())
        .collect()
}

fn changes_title(app: &mut App) -> String {
    let list = single::<ChangesList>(app);
    title_of(app, list)
}

/// The Compare page over the test plan, the scenario `text` compared with
/// it as `changed.toml` and highlighted.
fn highlighting(text: &str) -> crate::commands::tui::support::Headless {
    let dir = scratch_workspace(&test_plan_briefly_run());
    let overlay = format!("schema = 1\nbase = \"plan.toml\"\n{text}");
    std::fs::write(dir.join("changed.toml"), overlay).unwrap();
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::Compare);
    compare_with(&mut app, "changed");
    press_key(&mut app, KeyCode::Down);
    redrawn(&mut app);
    app
}

#[test]
fn the_pane_follows_the_highlighted_plan_and_the_baseline() {
    let mut app = comparing_variant(SIZE);
    assert_eq!(rows(&mut app), [THE_BASELINE]);
    assert_eq!(changes_title(&mut app), "Changes · plan.toml");
    press_key(&mut app, KeyCode::Down);
    redrawn(&mut app);
    assert_eq!(
        changes_title(&mut app),
        "Changes · variant.toml against plan.toml"
    );
    assert_eq!(
        rows(&mut app),
        ["Settings › Plan name: test-plan →", "  variant"],
        "a change wider than the pane runs on to an indented row"
    );
    press_key(&mut app, KeyCode::Char('b'));
    redrawn(&mut app);
    assert_eq!(rows(&mut app), [THE_BASELINE]);
    press_key(&mut app, KeyCode::Up);
    redrawn(&mut app);
    assert_eq!(
        changes_title(&mut app),
        "Changes · plan.toml against variant.toml"
    );
    assert_eq!(
        rows(&mut app),
        ["Settings › Plan name: variant →", "  test-plan"]
    );
}

#[test]
fn each_change_reads_in_the_forms_words() {
    let mut app = highlighting(
        r#"
[plan]
inflation = 0.03

[[income]]
id = "salary"
end = { age = 62, owner = "me" }

[[expenses]]
id = "living"
remove = true

[[expenses]]
id = "travel"
name = "Travel"
amount = 5000
"#,
    );
    assert_eq!(
        rows(&mut app),
        [
            "Settings › Inflation: 2.5% → 3%",
            "Income › salary › Ends: age 60 (me) →",
            "  age 62 (me)",
            "Expenses › living: removed",
            "Expenses › Travel: added",
        ]
    );
}

#[test]
fn a_plan_the_same_as_the_baseline_says_so() {
    let mut app = highlighting("");
    assert_eq!(rows(&mut app), [THE_SAME]);
}

#[test]
fn the_pane_is_a_stop_the_arrows_move_through() {
    let mut app = highlighting("[plan]\ninflation = 0.03\nhorizon_age = 72\n");
    let (table, list) = (
        single::<PlansTable>(&mut app),
        single::<ChangesList>(&mut app),
    );
    press_key(&mut app, KeyCode::Right);
    assert_eq!(
        focused(&app),
        Some(table),
        "→ walks the metric, not the row"
    );
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(focused(&app), Some(list));
    press_key(&mut app, KeyCode::Left);
    assert_eq!(focused(&app), Some(list), "and ← back");
    let cursor = |app: &App| app.world().get::<ActiveDescendant>(list).unwrap().0;
    let first = cursor(&app);
    press_key(&mut app, KeyCode::Down);
    assert_ne!(cursor(&app), first);
}

#[test]
fn the_pane_takes_a_third_of_the_row() {
    let list_cols = |size| {
        let mut app = comparing_variant(size);
        let list = single::<ChangesList>(&mut app);
        app.world().get::<ComputedWidgetArea>(list).unwrap().0.width
    };
    assert_eq!(
        (list_cols(SIZE), list_cols(ROOMY)),
        (41, 65),
        "43 and 67 columns, borders included"
    );
}

#[test]
fn a_wider_pane_joins_what_it_ran_on() {
    let mut app = comparing_variant(SIZE);
    press_key(&mut app, KeyCode::Down);
    redrawn(&mut app);
    app.insert_resource(ROOMY);
    redrawn(&mut app);
    assert_eq!(
        rows(&mut app),
        ["Settings › Plan name: test-plan → variant"]
    );
}

#[test]
fn a_named_item_is_read_by_its_name() {
    let mut app = highlighting(
        r#"
[[expenses]]
id = "travel"
name = "Travel"
amount = 5000
"#,
    );
    assert_eq!(rows(&mut app), ["Expenses › Travel: added"]);
}
