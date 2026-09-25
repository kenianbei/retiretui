//! Composed-frame snapshots: what each page draws, at the smallest
//! terminal the shell lays out in and at a full-screen one.

use bevy_app::App;
use plurimus::core::TerminalSize;
use plurimus::term::KeyCode;

use super::compare::Compared;
use super::edit::tests::open;
use super::nav::Page;
use super::session::Session;
use super::support::{
    FEW_TRIALS, Headless, ROOMY, SETTLING_TICKS, SIZE, commit_edit, composed_frame,
    headless_app_at, let_pass, press_ctrl, press_key, scratch_workspace, searched_app, show,
    type_text,
};
use super::tools::Searches;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../retiretui_engine/tests/fixtures/full.toml"
);

fn fixture_app(size: TerminalSize) -> Headless {
    let mut app = headless_app_at(FIXTURE.into(), size);
    app.update();
    app
}

fn fixture_text() -> String {
    std::fs::read_to_string(FIXTURE).unwrap()
}

/// The full fixture under its own name, its searches answered.
fn searched_fixture(size: TerminalSize) -> Headless {
    let path = scratch_workspace("").join("full.toml");
    searched_app(path, &fixture_text(), size)
}

/// Named for the size it was drawn at, so a snapshot cannot outlive the
/// constant that sized it.
fn assert_frame(name: &str, size: TerminalSize, app: &App) {
    let name = format!("{name}_{}x{}", size.cols, size.rows);
    insta::assert_snapshot!(name, composed_frame(app));
}

#[test]
fn every_page_draws_its_frame() {
    for size in [SIZE, ROOMY] {
        let mut app = fixture_app(size);
        app.insert_resource(Searches(true));
        for page in Page::ALL {
            show(&mut app, page);
            // A tool's page searches on a thread of its own; its frame is
            // the one the answer leaves.
            super::tools::settle_all(&mut app);
            assert_frame(page.label(), size, &app);
        }
    }
}

/// The Overview's chart in each view `v` turns to after the balances.
#[test]
fn the_overview_draws_each_chart_view() {
    for size in [SIZE, ROOMY] {
        let mut app = fixture_app(size);
        for view in ["net_worth", "income_taxes"] {
            press_key(&mut app, KeyCode::Char('v'));
            app.update();
            assert_frame(&format!("overview_{view}"), size, &app);
        }
    }
}

/// The Overview of a plan that runs short and pays into its 401(k) past
/// the limit.
#[test]
fn the_overview_draws_what_needs_attention() {
    let plan = super::support::TEST_PLAN.replace("amount = 60000", "amount = 90000");
    let plan =
        format!("{plan}\n[[contributions]]\nid = \"deferral\"\nto = \"k\"\namount = 50000\n");
    for size in [SIZE, ROOMY] {
        let path = scratch_workspace(&plan).join("plan.toml");
        let app = searched_app(path, &plan, size);
        assert_frame("overview_attention", size, &app);
    }
}

/// The form a new plan is composed in, with no document beneath it.
#[test]
fn the_new_plan_form_draws_its_frame() {
    for size in [SIZE, ROOMY] {
        let mut app = headless_app_at(scratch_workspace(super::support::TEST_PLAN), size);
        press_key(&mut app, KeyCode::Esc);
        for _ in 0..SETTLING_TICKS {
            app.update();
        }
        assert_frame("new_plan", size, &app);
    }
}

#[test]
fn an_open_item_draws_its_overlay() {
    for size in [SIZE, ROOMY] {
        let mut app = fixture_app(size);
        open(&mut app, Page::Accounts);
        let frame = composed_frame(&app);
        assert!(frame.contains("Edit cash"), "the item is open: {frame}");
        assert_frame("accounts_item", size, &app);
    }
}

#[test]
fn an_open_select_draws_its_menu_over_the_form() {
    let mut app = fixture_app(ROOMY);
    open(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Char(' '));
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains(" SIMPLE IRA "), "the menu is open: {frame}");
    assert_frame("accounts_select", ROOMY, &app);
}

#[test]
fn a_question_draws_its_dialog() {
    let mut app = fixture_app(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Char('d'));
    app.update();
    assert_frame("accounts_delete", SIZE, &app);
}

#[test]
fn the_picker_draws_over_the_page() {
    let mut app = searched_fixture(SIZE);
    press_key(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "page");
    app.update();
    assert_frame("commands_picker", SIZE, &app);
}

#[test]
fn the_workspace_picker_draws_over_the_empty_shell_and_over_a_page() {
    let dir = scratch_workspace(&fixture_text());

    for size in [SIZE, ROOMY] {
        let mut app = headless_app_at(dir.clone(), size);
        app.update();
        app.update();
        if size == SIZE {
            assert_frame("open_picker_empty", size, &app);
        }
        press_key(&mut app, KeyCode::Esc);
        assert_frame("empty_shell", size, &app);
    }

    let mut app = searched_app(dir.join("plan.toml"), &fixture_text(), SIZE);
    press_ctrl(&mut app, KeyCode::Char('o'));
    app.update();
    assert_frame("open_picker", SIZE, &app);
}

#[test]
fn the_issues_panel_draws_over_the_page() {
    let mut app = searched_fixture(SIZE);
    commit_edit(&mut app, |plan| {
        plan.accounts[1].balance = -1;
        plan.plan.inflation = 9.0;
    });
    press_key(&mut app, KeyCode::Char('i'));
    app.update();
    assert_frame("issues", SIZE, &app);
}

#[test]
fn the_statement_picker_draws_over_the_people_page() {
    let mut app = fixture_app(SIZE);
    show(&mut app, Page::People);
    press_key(&mut app, KeyCode::Char('e'));
    app.update();
    assert_frame("people_import", SIZE, &app);
}

/// Each view `v` turns a market tool's chart pane to.
#[test]
fn every_market_view_draws_its_frame() {
    for size in [SIZE, ROOMY] {
        let mut app = fixture_app(size);
        for (page, views) in [
            (
                Page::MonteCarlo,
                ["by_year", "still_funded", "endings"].as_slice(),
            ),
            (Page::Historical, ["still_funded", "endings"].as_slice()),
        ] {
            show(&mut app, page);
            super::tools::settle_all(&mut app);
            for view in views {
                press_key(&mut app, KeyCode::Char('v'));
                assert_frame(&format!("{}_{view}", page.label()), size, &app);
            }
            press_key(&mut app, KeyCode::Char('v'));
        }
    }
}

/// The Compare page over the full fixture, the scenario `text` compared
/// with it as `name`.
fn comparing(size: TerminalSize, name: &str, text: &str) -> Headless {
    let plan = fixture_text() + FEW_TRIALS;
    let dir = scratch_workspace(&plan);
    std::fs::write(dir.join(name), text).unwrap();
    let mut app = headless_app_at(dir.join("plan.toml"), size);
    show(&mut app, Page::Compare);
    let tables = app.world().resource::<Session>().tables.clone();
    let mut compared = app.world_mut().resource_mut::<Compared>();
    compared.take_in(dir.join(name), &tables).unwrap();
    app
}

fn settle(app: &mut App) {
    for _ in 0..SETTLING_TICKS {
        app.update();
    }
    super::tools::settle_all(app);
}

const RICHER: &str =
    "schema = 1\nbase = \"plan.toml\"\n\n[[accounts]]\nid = \"brokerage\"\nbalance = 900000\n";

/// The Compare page with a scenario over the document beside it, in
/// each view `v` turns to.
#[test]
fn a_compared_plan_draws_its_row_and_line() {
    for size in [SIZE, ROOMY] {
        let mut app = comparing(size, "richer.toml", RICHER);
        settle(&mut app);
        assert_frame("compare_one", size, &app);
        for view in ["grid", "by_year"] {
            press_key(&mut app, KeyCode::Char('v'));
            assert_frame(&format!("compare_one_{view}"), size, &app);
        }
    }
}

/// The Compare page under difference, a poorer scenario beside the
/// document, so its line runs below zero.
#[test]
fn a_poorer_plan_under_difference_draws_below_zero() {
    let poorer = "schema = 1\nbase = \"plan.toml\"\n\n[[accounts]]\nid = \"brokerage\"\nbalance = 100000\nbasis = 80000\n";
    for size in [SIZE, ROOMY] {
        let mut app = comparing(size, "poorer.toml", poorer);
        press_key(&mut app, KeyCode::Char('d'));
        settle(&mut app);
        assert_frame("compare_difference", size, &app);
    }
}

/// A compared file broken on disk after it was read: its row keeps its
/// figures, named in the colour of what went wrong, and the pane says why.
#[test]
fn a_compared_file_that_fails_to_read_again_is_marked() {
    let mut app = comparing(SIZE, "richer.toml", RICHER);
    let workspace = app.world().resource::<Session>().workspace().to_path_buf();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let broken = format!("{RICHER}\n[plan]\ninflation = 9.0\n");
    std::fs::write(workspace.join("richer.toml"), broken).unwrap();
    let_pass(
        &mut app,
        std::time::Duration::from_secs_f32(super::watch::POLL_SECONDS),
    );
    settle(&mut app);
    assert_frame("compare_marked", SIZE, &app);
}

const CHANGED: &str = r#"schema = 1
base = "plan.toml"

[[income]]
id = "ss-jordan"
start = { age = 70, owner = "jordan" }

[[expenses]]
id = "travel"
remove = true

[[conversions]]
id = "roth-2027"
from = "fid-401k"
to = "roth-ira"
amount = 20000
start = { date = 2027-01-01 }
end = { date = 2030-01-01 }
"#;

/// The Changes pane on a scenario over the document: an item changed,
/// one removed and one added.
#[test]
fn a_compared_scenario_lists_what_it_changes() {
    for size in [SIZE, ROOMY] {
        let mut app = comparing(size, "changed.toml", CHANGED);
        app.update();
        press_key(&mut app, KeyCode::Down);
        settle(&mut app);
        assert_frame("compare_changes", size, &app);
    }
}
