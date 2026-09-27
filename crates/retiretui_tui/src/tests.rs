//! Headless shell behavior tests.

use std::collections::BTreeSet;
use std::time::Duration;

use bevy_app::App;
use bevy_ui::{Display, Node};
use plurimus::core::TerminalSize;
use plurimus::term::KeyCode;

use bevy_input_focus::InputFocus;
use retiretui_engine::plan::Plan;

use super::command;
use super::edit::Draft;
use super::layout;
use super::nav::surface::SurfaceRoot;
use super::nav::{self, Page};
use super::picker_tests::is_palette_open;
use super::session::{NO_DOCUMENT, Session, YearCursor};
use super::support::{
    self, ROOMY, SIZE, active_page as active, change_on_disk, commit_edit, composed_frame,
    headless_app, headless_app_at, is_browsing, let_pass, press_ctrl, press_key, said, show,
};

/// The pages whose root takes room in the body.
fn shown_pages(app: &mut App) -> BTreeSet<Page> {
    let world = app.world_mut();
    let mut roots = world.query::<(&SurfaceRoot, &Node)>();
    roots
        .iter(world)
        .filter(|(_, node)| node.display != Display::None)
        .filter_map(|(root, _)| root.0)
        .collect()
}

fn plan_name(app: &App) -> String {
    let projected = app.world().resource::<super::session::Projected>();
    projected.plan.plan.name.clone().unwrap_or_default()
}

#[test]
fn shell_draws_the_active_page_and_the_hint_row() {
    let mut app = headless_app(SIZE);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("1 Overview"), "the tab bar: {frame}");
    assert!(frame.contains("Balances by tax treatment"), "{frame}");
    assert!(frame.contains("q quit"), "{frame}");
    assert_eq!(shown_pages(&mut app), BTreeSet::from([Page::Overview]));
}

#[test]
fn a_directory_launches_the_shell_empty() {
    let plan = support::scratch_plan();
    let mut app = headless_app_at(plan.parent().unwrap().to_path_buf(), SIZE);
    assert!(is_browsing(&app), "asking which file to open");
    press_key(&mut app, KeyCode::Esc);
    let frame = composed_frame(&app);
    assert!(frame.contains("no document"), "{frame}");
    assert!(!frame.contains("Income / Taxes"), "{frame}");
    assert!(
        frame.contains("New plan") && frame.contains("Filing status"),
        "and offers to build one: {frame}"
    );
    assert!(shown_pages(&mut app).is_empty(), "no page of a plan");
    assert!(
        app.world().resource::<InputFocus>().get().is_some(),
        "the new plan's form takes the keyboard"
    );
    assert_eq!(
        support::lit_tab(&mut app),
        Some(nav::Group::Plan.tab()),
        "the one tab with anything to say"
    );
    let before = active(&app);
    support::invoke(&mut app, "tab-next");
    assert_eq!(active(&app), before, "and the others refuse to be shown");
    assert_eq!(said(&app).last().map(String::as_str), Some(NO_DOCUMENT));
    show(&mut app, Page::Accounts);
    assert!(
        shown_pages(&mut app).is_empty(),
        "a page shown draws nothing"
    );
    let held = app.world().resource::<InputFocus>().get();
    let is_form = held.is_some_and(|held| app.world().get::<super::edit::EditForm>(held).is_some());
    assert!(is_form, "and its table takes no keyboard from the form");
    let frame = composed_frame(&app);
    assert!(!frame.contains("save"), "no hint for what refuses: {frame}");
    support::run_command(&mut app, "save");
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some(NO_DOCUMENT),
        "saving is refused rather than run"
    );
    press_key(&mut app, KeyCode::Char(':'));
    assert!(is_palette_open(&app), "the palette still opens");
}

#[test]
fn every_command_that_needs_a_document_refuses_without_one() {
    let mut app = headless_app_at(support::scratch_dir(), SIZE);
    let setup = None;
    let refused: Vec<_> = command::all()
        .filter(|command| !command.spec().scope.covers(setup))
        .collect();
    for &command in &refused {
        support::invoke(&mut app, command.spec().name);
        assert_eq!(
            said(&app).last().map(String::as_str),
            Some(NO_DOCUMENT),
            "{}",
            command.spec().name
        );
    }
    let refusals = said(&app)
        .iter()
        .filter(|line| *line == NO_DOCUMENT)
        .count();
    assert_eq!(refusals, refused.len());
}

#[test]
fn tab_keys_step_through_every_tab_and_wrap() {
    let mut app = headless_app(SIZE);
    press_ctrl(&mut app, KeyCode::Down);
    assert_eq!(active(&app), Page::Ledger);
    assert_eq!(shown_pages(&mut app), BTreeSet::from([Page::Ledger]));
    for _ in 1..nav::TAB_COUNT {
        press_ctrl(&mut app, KeyCode::Down);
    }
    assert_eq!(active(&app), Page::Overview, "the last tab wraps");
    press_ctrl(&mut app, KeyCode::Up);
    assert_eq!(active(&app), Page::Accounts, "and backwards, to the plan");
    assert!(frame_of(&mut app).contains("Conversions"));
}

fn frame_of(app: &mut App) -> String {
    app.update();
    composed_frame(app)
}

#[test]
fn q_exits() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char('q'));
    assert!(app.should_exit().is_some());
}

#[test]
fn small_terminals_get_only_the_notice() {
    let mut app = headless_app(TerminalSize::new(40, 10));
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("terminal too small"), "{frame}");
    assert!(!frame.contains("q quit"), "{frame}");
    assert!(!frame.contains("Income / Taxes"), "{frame}");
}

fn resize(app: &mut App, size: TerminalSize) {
    app.insert_resource(size);
    app.update();
    app.update();
}

#[test]
fn the_chrome_keeps_the_first_and_last_rows_through_a_resize() {
    let mut app = headless_app(SIZE);
    app.update();
    for size in [SIZE, TerminalSize::new(160, 40)] {
        resize(&mut app, size);
        let frame = composed_frame(&app);
        let rows: Vec<&str> = frame.lines().collect();
        assert_eq!(rows.len(), usize::from(size.rows));
        assert!(rows[0].contains('\u{256d}'), "the tab row: {frame}");
        assert!(rows[rows.len() - 1].contains("q quit"), "{frame}");
        assert!(
            rows[usize::from(layout::BODY_TOP)].contains("Lifetime taxes"),
            "the body starts under the tab row: {frame}"
        );
    }
    resize(&mut app, TerminalSize::new(40, 10));
    assert!(!composed_frame(&app).contains("q quit"));
    resize(&mut app, SIZE);
    let frame = composed_frame(&app);
    assert!(frame.lines().last().unwrap().contains("q quit"), "{frame}");
    assert!(!frame.contains("terminal too small"), "{frame}");
}

#[test]
fn the_overview_renders_its_verdict_chart_and_to_do() {
    let mut app = headless_app(SIZE);
    app.update();
    let frame = composed_frame(&app);
    for said in ["Money lasts", "Success", "Ends with", "Lifetime taxes"] {
        assert!(frame.contains(said), "{said}: {frame}");
    }
    assert!(
        frame.contains("Balances by tax treatment · today's dollars"),
        "{frame}"
    );
    assert!(
        frame.contains("2026 · to do · in that year's dollars"),
        "the To do is always nominal: {frame}"
    );
}

#[test]
fn ledger_lists_years_and_detail_follows_the_cursor() {
    // Roomy, so the detail's last lines are above its fold.
    let mut app = headless_app(ROOMY);
    show(&mut app, Page::Ledger);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("2026"), "{frame}");
    assert!(frame.contains("Net worth"), "{frame}");
    assert!(
        frame.contains("2026 Flows · today's dollars"),
        "the pane names the cursor year and its dollars: {frame}"
    );
    assert!(frame.contains("salary"), "{frame}");
    assert!(frame.contains("Spending"), "{frame}");
    assert!(
        !frame.contains("Unfunded"),
        "nothing unfunded, so no line for it: {frame}"
    );
    assert!(frame.contains("+$26,830 surplus"), "{frame}");
    press_key(&mut app, KeyCode::Down);
    app.update();
    assert_eq!(
        app.world().resource::<YearCursor>().0,
        Some(2027),
        "arrow moves the cursor off the first year"
    );
    let frame = composed_frame(&app);
    assert!(frame.contains("2027 Flows"), "{frame}");
    press_key(&mut app, KeyCode::Tab);
    let frame = composed_frame(&app);
    assert!(frame.contains("↑↓ account"), "⇥ reaches the flows: {frame}");
    press_key(&mut app, KeyCode::Tab);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("↑↓ line"),
        "and then income and tax: {frame}"
    );
}

#[test]
fn the_ledger_keeps_its_rows_after_a_visit_elsewhere() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Ledger);
    app.update();
    show(&mut app, Page::Overview);
    app.update();
    show(&mut app, Page::Ledger);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("Net worth"), "the header survives: {frame}");
    assert!(frame.contains("2026"), "the first year survives: {frame}");
}

#[test]
fn r_reloads_edits_and_keeps_the_view_on_failure() {
    let mut app = headless_app(SIZE);
    let plan_path = app.world().resource::<Session>().plan_path.clone().unwrap();
    let renamed = support::TEST_PLAN.replace("name = \"test-plan\"", "name = \"edited-plan\"");
    std::fs::write(&plan_path, renamed).unwrap();
    press_key(&mut app, KeyCode::Char('r'));
    app.update();
    let frame = composed_frame(&app);
    assert_eq!(plan_name(&app), "edited-plan");
    assert!(frame.contains("plan reloaded"), "{frame}");
    let broken = support::TEST_PLAN.replace("inflation = 0.025", "inflation = 9.0");
    std::fs::write(&plan_path, broken).unwrap();
    press_key(&mut app, KeyCode::Char('r'));
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("reload failed"), "{frame}");
    assert_eq!(plan_name(&app), "edited-plan", "last good view survives");
}

#[test]
fn a_disk_change_waits_behind_an_unsaved_draft() {
    let mut app = headless_app(SIZE);
    let plan_path = app.world().resource::<Session>().plan_path.clone().unwrap();
    edit_draft(&mut app, |plan| {
        plan.plan.name = Some("draft-plan".to_owned());
    });
    let renamed = support::TEST_PLAN.replace("name = \"test-plan\"", "name = \"edited-plan\"");
    change_on_disk(&mut app, &plan_path, &renamed);
    let_pass(
        &mut app,
        Duration::from_secs_f32(super::watch::POLL_SECONDS),
    );
    assert_eq!(
        plan_name(&app),
        "draft-plan",
        "the draft is not overwritten"
    );
    let warned = said(&app)
        .iter()
        .filter(|text| text.contains("plan changed on disk"))
        .count();
    assert_eq!(warned, 1, "said once, not every beat");
}

#[test]
fn a_disk_change_is_followed_on_the_beat() {
    let mut app = headless_app(SIZE);
    let plan_path = app.world().resource::<Session>().plan_path.clone().unwrap();
    let renamed = support::TEST_PLAN.replace("name = \"test-plan\"", "name = \"edited-plan\"");
    change_on_disk(&mut app, &plan_path, &renamed);
    assert_eq!(plan_name(&app), "edited-plan");
}

#[test]
fn a_failed_reload_watches_the_chain_as_it_now_reads() {
    let mut app = headless_app(SIZE);
    let plan_path = app.world().resource::<Session>().plan_path.clone().unwrap();
    let base = plan_path.with_file_name("base.toml");
    let broken = support::TEST_PLAN.replace("inflation = 0.025", "inflation = 9.0");
    std::fs::write(&base, broken).unwrap();
    change_on_disk(&mut app, &plan_path, &support::scenario_over("base.toml"));
    assert!(
        said(&app).iter().any(|text| text.contains("reload failed")),
        "{:?}",
        said(&app)
    );
    change_on_disk(&mut app, &base, support::TEST_PLAN);
    assert_eq!(plan_name(&app), "variant", "fixing the new base reloads");
}

#[test]
fn n_toggles_the_dollar_basis() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char('n'));
    let frame = composed_frame(&app);
    assert!(frame.contains("future dollars"), "{frame}");
    press_key(&mut app, KeyCode::Char('n'));
    let frame = composed_frame(&app);
    assert!(frame.contains("today's dollars"), "{frame}");
}

fn edit_draft(app: &mut App, edit: impl Fn(&mut Plan) + Send + Sync + 'static) {
    commit_edit(app, edit);
    app.update();
    app.update();
}

#[test]
fn draft_edits_reproject_live_and_save_canonically() {
    let mut app = headless_app(SIZE);
    let plan_path = app.world().resource::<Session>().plan_path.clone().unwrap();
    edit_draft(&mut app, |plan| {
        plan.plan.name = Some("draft-plan".to_owned());
    });
    assert_eq!(plan_name(&app), "draft-plan", "live re-projection");
    assert!(
        std::fs::read_to_string(&plan_path)
            .unwrap()
            .contains("test-plan"),
        "nothing written before save"
    );
    edit_draft(&mut app, |plan| plan.plan.inflation = 9.0);
    let frame = composed_frame(&app);
    assert_eq!(
        plan_name(&app),
        "draft-plan",
        "invalid draft holds the view"
    );
    assert!(
        frame.contains("Settings › Inflation"),
        "first issue shown: {frame}"
    );
    press_ctrl(&mut app, KeyCode::Char('s'));
    let frame = composed_frame(&app);
    assert!(frame.contains("not saved, 1 issue"), "{frame}");
    edit_draft(&mut app, |plan| plan.plan.inflation = 0.03);
    press_ctrl(&mut app, KeyCode::Char('s'));
    let frame = composed_frame(&app);
    assert!(frame.contains("saved "), "{frame}");
    let written = std::fs::read_to_string(&plan_path).unwrap();
    let expected = app
        .world()
        .resource::<Draft>()
        .plan
        .to_toml_string()
        .unwrap();
    assert_eq!(written, expected, "canonical TOML on disk");
    press_key(&mut app, KeyCode::Char('r'));
    let frame = composed_frame(&app);
    assert!(frame.contains("plan reloaded"), "{frame}");
    assert_eq!(plan_name(&app), "draft-plan");
    press_ctrl(&mut app, KeyCode::Char('s'));
    assert_eq!(
        std::fs::read_to_string(&plan_path).unwrap(),
        written,
        "saving again is byte-identical"
    );
}

#[test]
fn quitting_a_dirty_draft_asks_first() {
    let mut app = headless_app(SIZE);
    edit_draft(&mut app, |plan| {
        plan.plan.name = Some("draft-plan".to_owned());
    });
    press_key(&mut app, KeyCode::Char('q'));
    assert!(app.should_exit().is_none());
    let frame = composed_frame(&app);
    assert!(frame.contains("lose the unsaved changes"), "{frame}");
    press_key(&mut app, KeyCode::Char('q'));
    assert!(app.should_exit().is_none(), "the dialog owns the keyboard");
    press_key(&mut app, KeyCode::Esc);
    assert!(app.should_exit().is_none());
    press_key(&mut app, KeyCode::Char('q'));
    press_key(&mut app, KeyCode::Enter);
    assert!(app.should_exit().is_some());
}

#[test]
fn scenario_sessions_refuse_to_save() {
    let scenario = support::scratch_scenario();
    let mut app = headless_app_at(scenario.clone(), SIZE);
    press_ctrl(&mut app, KeyCode::Char('s'));
    let frame = composed_frame(&app);
    assert!(frame.contains("read-only"), "{frame}");
    assert!(
        std::fs::read_to_string(&scenario)
            .unwrap()
            .contains("base =")
    );
}

#[test]
fn the_detail_is_as_tall_as_its_year_needs_up_to_half_the_page() {
    for size in [SIZE, ROOMY] {
        let mut app = headless_app(size);
        show(&mut app, Page::Ledger);
        let detail_of = |frame: &str| {
            let lines: Vec<&str> = frame.lines().collect();
            let starts = |title: &str| lines.iter().position(|line| line.contains(title)).unwrap();
            let (table_top, detail_top) = (starts("╭ Ledger"), starts("Flows ·"));
            let key_row = lines.len() - 1;
            (key_row - detail_top, key_row - table_top)
        };
        let (working, whole) = detail_of(&composed_frame(&app));
        assert!(
            working * 2 <= whole,
            "{working} of {whole} rows at {size:?}"
        );
        press_key(&mut app, KeyCode::End);
        let (retired, _) = detail_of(&composed_frame(&app));
        assert!(
            retired < working,
            "a quieter year takes fewer rows: {retired} against {working} at {size:?}"
        );
    }
}
