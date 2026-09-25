use bevy_app::App;
use bevy_ecs::prelude::With;
use plurimus::core::TerminalSize;
use plurimus::term::KeyCode;

use super::{DIGIT_COLS, TAB_DECORATION, TABS_COLS, look};
use crate::commands::tui::edit::Draft;
use crate::commands::tui::layout::TAB_ROW_ROWS;
use crate::commands::tui::nav::{ActivePage, Group, Page, TAB_COUNT, tab_digit, tab_title};
use crate::commands::tui::support::{
    ROOMY, SIZE, active_page, cell_style, click, commit_edit, headless_app, headless_app_at,
    press_key, redrawn, run_command, scratch_plan,
};

#[test]
fn the_row_the_frame_reserves_is_the_look_the_bar_draws() {
    assert_eq!(look().thickness(), TAB_ROW_ROWS);
}

#[test]
fn the_bar_names_every_tab_with_the_digit_that_selects_it() {
    let mut app = headless_app(SIZE);
    let frame = redrawn(&mut app);
    for tab in 0..TAB_COUNT {
        let named = format!("{} {}", tab_digit(tab), tab_title(tab));
        assert!(frame.contains(&named), "tab {tab}: {frame}");
    }
    let boxes: Vec<char> = frame
        .lines()
        .next()
        .expect("the bar's top row")
        .chars()
        .collect();
    let last = boxes.iter().rposition(|glyph| *glyph == '\u{256e}');
    assert_eq!(
        last.map(|at| at + 1),
        Some(usize::from(TABS_COLS)),
        "the columns the frame reserves are the ones the bar draws: {frame}"
    );
}

#[test]
fn a_digit_selects_the_tab_it_names() {
    let mut app = headless_app(SIZE);
    for tab in 0..TAB_COUNT {
        press_key(&mut app, KeyCode::Char(tab_digit(tab)));
        assert_eq!(active_page(&app).tab(), tab, "tab {tab}");
    }
}

#[test]
fn the_bar_lights_the_tab_the_page_on_show_is_reached_through() {
    let mut app = headless_app(SIZE);
    assert_eq!(lit_tab(&mut app), Some(Page::Overview.tab()));
    app.insert_resource(ActivePage(Page::Expenses));
    app.update();
    app.update();
    assert_eq!(
        lit_tab(&mut app),
        Some(Page::Expenses.tab()),
        "a domain lights the Plan tab"
    );
    run_command(&mut app, "compare");
    app.update();
    assert_eq!(lit_tab(&mut app), Some(Page::Compare.tab()));
}

#[test]
fn choosing_a_tab_with_the_pointer_shows_its_page() {
    let mut app = headless_app(ROOMY);
    click(&mut app, tab_column(1), TAB_ROW_ROWS / 2);
    assert_eq!(active_page(&app), Page::Ledger);
}

#[test]
fn the_status_names_the_document_and_cuts_the_name_where_the_tabs_leave_no_room() {
    let short = scratch_plan();
    let stem = short.file_stem().expect("a named plan").to_string_lossy();
    let long = short.with_file_name(format!("{stem}-{}.toml", "long-name".repeat(6)));
    std::fs::rename(&short, &long).expect("a scratch plan to rename");
    let mut app = headless_app_at(long, ROOMY);
    let named = redrawn(&mut app);
    let row = status_row(&named);
    assert!(row.contains(".toml"), "the document is named: {named}");
    assert!(row.contains('\u{25cf}'), "and marked: {named}");

    commit_edit(&mut app, |plan| plan.plan.start_year = 1);
    let counted = app.world().resource::<Draft>().issues().len();
    let broken = redrawn(&mut app);
    let said = format!("{counted} issues");
    assert!(
        status_row(&broken).contains(&said),
        "the count follows the draft: {broken}"
    );

    *app.world_mut().resource_mut::<TerminalSize>() = SIZE;
    let narrow = redrawn(&mut app);
    let row = status_row(&narrow);
    let cut = format!("\u{2026} \u{25cf} {said}");
    assert!(row.contains(&cut), "the name is cut, not dropped: {narrow}");
    assert!(row.contains(&*stem), "from its end: {narrow}");
    assert_eq!(row.chars().count(), usize::from(SIZE.cols), "{narrow}");
}

#[test]
fn the_baseline_runs_under_the_status_to_the_edge() {
    let mut app = headless_app(SIZE);
    let frame = redrawn(&mut app);
    let foot = frame
        .lines()
        .nth(usize::from(TAB_ROW_ROWS - 1))
        .expect("the bar's foot");
    assert_eq!(foot.chars().count(), usize::from(SIZE.cols), "{frame}");
    let past_the_tabs: String = foot.chars().skip(usize::from(TABS_COLS)).collect();
    assert!(
        past_the_tabs.chars().all(|cell| cell == '\u{2500}'),
        "no gap under the status: {frame}"
    );
    let row = TAB_ROW_ROWS - 1;
    assert_eq!(
        cell_style(&app, SIZE.cols - 1, row),
        cell_style(&app, TABS_COLS, row),
        "and no seam where the bar's own baseline ends"
    );
}

#[test]
fn without_a_document_only_the_plan_tab_is_live() {
    let mut app = headless_app_at(std::env::temp_dir(), SIZE);
    press_key(&mut app, KeyCode::Esc);
    app.update();
    let dead = dead_tabs(&mut app);
    assert_eq!(dead.len(), TAB_COUNT - 1, "every tab but Plan: {dead:?}");
    assert!(!dead.contains(&Group::Plan.tab()), "{dead:?}");
    assert_eq!(active_page(&app), Page::Accounts);

    let ledger = Page::Ledger.tab();
    click(&mut app, tab_column(ledger), TAB_ROW_ROWS / 2);
    assert_eq!(
        active_page(&app),
        Page::Accounts,
        "a dead tab answers to nothing"
    );
}

/// The tabs the bar will not act on.
fn dead_tabs(app: &mut App) -> Vec<usize> {
    let world = app.world_mut();
    let mut items =
        world.query_filtered::<&super::BarTab, With<plurimus::ui::InteractionDisabled>>();
    items.iter(world).map(|tab| tab.0).collect()
}

/// The row of the tab box the status is centred on.
fn status_row(frame: &str) -> &str {
    frame
        .lines()
        .nth(usize::from(TAB_ROW_ROWS / 2))
        .expect("the bar's middle row")
}

/// The tab the bar draws as active, read back off its items.
fn lit_tab(app: &mut App) -> Option<usize> {
    let world = app.world_mut();
    let mut items = world.query_filtered::<&super::BarTab, With<plurimus::ui::Checked>>();
    items.iter(world).map(|tab| tab.0).next()
}

/// A column inside the `tab`th box, counted off the labels before it.
fn tab_column(tab: usize) -> u16 {
    let boxed = |at: usize| tab_title(at).chars().count() as u16 + DIGIT_COLS + TAB_DECORATION;
    (0..tab).map(boxed).sum::<u16>() + 1
}

#[test]
fn without_a_document_the_status_drops_the_mark_and_keeps_the_count() {
    let mut app = headless_app_at(std::env::temp_dir(), SIZE);
    press_key(&mut app, KeyCode::Esc);
    let frame = redrawn(&mut app);
    let row = status_row(&frame);
    assert!(row.contains("no document"), "{frame}");
    assert!(
        !row.contains('\u{25cf}'),
        "the mark claims a draft differs from a file there is none of: {frame}"
    );

    commit_edit(&mut app, |plan| plan.plan.start_year = 1);
    let counted = app.world().resource::<Draft>().issues().len();
    let broken = redrawn(&mut app);
    assert!(
        status_row(&broken).contains(&format!("{counted} issues")),
        "what is being composed is still checked: {broken}"
    );
}
