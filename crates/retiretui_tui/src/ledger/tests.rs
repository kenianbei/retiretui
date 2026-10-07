//! The Ledger as a user drives it: its two views, its two shapes, the
//! table's column sets, and the year stepped from any pane.

use bevy_app::App;
use bevy_ecs::prelude::With;
use bevy_input_focus::InputFocus;
use plurimus::core::TerminalSize;
use plurimus::term::KeyCode;
use plurimus::widgets::TableStripe;

use super::{Columns, LedgerTable, LedgerView};
use crate::nav::Page;
use crate::session::{LedgerRun, Projected, YearCursor};
use crate::support::{
    ROOMY, SETTLING_TICKS, SIZE, TALL, TODAY, active_page, composed_frame, headless_app_at,
    ledger_year, press_key, redrawn, said, scratch_full_plan, show,
};
use crate::theme::Theme;
use crate::theme::document::Variant;
use retiretui_client::ledger::ColumnSet;

/// The full fixture on the Ledger, on a terminal of `size`: several
/// accounts, milestones and a ladder of conversions.
fn ledger_at(size: TerminalSize) -> crate::support::Headless {
    let mut app = headless_app_at(scratch_full_plan(), size);
    show(&mut app, Page::Ledger);
    app
}

/// The fixture on a landscape terminal with the room for the year.
fn ledger() -> crate::support::Headless {
    ledger_at(ROOMY)
}

fn view(app: &App) -> LedgerView {
    *app.world().resource::<LedgerView>()
}

fn cursor(app: &App) -> Option<i16> {
    app.world().resource::<YearCursor>().0
}

fn holds_the_table(app: &App) -> bool {
    let held = app.world().resource::<InputFocus>().get();
    held.is_some_and(|held| app.world().get::<LedgerTable>(held).is_some())
}

#[test]
fn t_gives_the_table_the_page_and_the_year_back_at_the_same_year() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::Down);
    for _ in 0..3 {
        press_key(&mut app, KeyCode::Tab);
    }
    assert!(composed_frame(&app).contains("↑↓ account"), "the flows");
    assert!(!holds_the_table(&app));
    press_key(&mut app, KeyCode::Char('t'));
    assert_eq!(view(&app), LedgerView::Table);
    assert!(holds_the_table(&app), "the keyboard goes with the pane");
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("╭ Ledger · today's dollars · Balances by treatment "),
        "{frame}"
    );
    assert!(
        !frame.contains("╭ Flows "),
        "the year gives up the page: {frame}"
    );
    assert!(frame.contains("Withdrawn"), "{frame}");
    assert!(frame.contains("c columns  t the year"), "{frame}");
    assert_eq!(ledger_year(&mut app), TODAY.0 + 1);
    press_key(&mut app, KeyCode::Tab);
    assert!(holds_the_table(&app), "the table is the only pane to walk");
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('t'));
    assert_eq!(view(&app), LedgerView::Year);
    assert!(holds_the_table(&app), "which the keyboard stays in");
    let frame = redrawn(&mut app);
    assert!(frame.contains("╭ Flows "), "{frame}");
    let to_do = format!("╭ To do in {} ", TODAY.0 + 2);
    assert!(frame.contains(&to_do), "{frame}");
    assert!(frame.contains("t table  c columns"), "{frame}");
}

#[test]
fn enter_in_the_table_shows_the_year_its_cursor_is_on() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(view(&app), LedgerView::Year, "⏎ over the year keeps it");
    press_key(&mut app, KeyCode::Char('t'));
    for _ in 0..3 {
        press_key(&mut app, KeyCode::Down);
    }
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(view(&app), LedgerView::Year);
    let frame = redrawn(&mut app);
    let to_do = format!("╭ To do in {} ", TODAY.0 + 3);
    assert!(frame.contains(&to_do), "{frame}");
}

#[test]
fn c_turns_the_table_through_its_column_sets() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('c'));
    assert_eq!(
        view(&app),
        LedgerView::Year,
        "c turns the table where it is"
    );
    let sets = [
        (ColumnSet::Accounts, "Balances by account", "fid-401k"),
        (ColumnSet::Tax, "Tax figures", "MAGI"),
        (ColumnSet::Treatments, "Balances by treatment", "Deferred"),
    ];
    for (set, title, header) in sets {
        assert_eq!(app.world().resource::<Columns>().0, set);
        let frame = redrawn(&mut app);
        assert!(frame.contains(&format!("· {title} ")), "{frame}");
        assert!(frame.contains(header), "{set:?}: {frame}");
        assert!(frame.contains("Withdraw"), "every set leads alike: {frame}");
        assert!(frame.contains("╭ Flows "), "over the year: {frame}");
        assert_eq!(ledger_year(&mut app), TODAY.0 + 1, "the year stays");
        press_key(&mut app, KeyCode::Char('c'));
    }
}

#[test]
fn the_year_is_stepped_from_any_pane_by_the_arrows_or_the_brackets() {
    let mut app = ledger();
    for _ in 0..3 {
        press_key(&mut app, KeyCode::Tab);
    }
    assert!(composed_frame(&app).contains("↑↓ account"), "the flows");
    press_key(&mut app, KeyCode::Right);
    assert_eq!(cursor(&app), Some(TODAY.0 + 1), "→ from the flows");
    press_key(&mut app, KeyCode::Char(']'));
    assert_eq!(cursor(&app), Some(TODAY.0 + 2));
    press_key(&mut app, KeyCode::Left);
    press_key(&mut app, KeyCode::Char('['));
    assert_eq!(cursor(&app), Some(TODAY.0));
    press_key(&mut app, KeyCode::Left);
    assert_eq!(cursor(&app), Some(TODAY.0), "held at the plan's first year");
    let frame = redrawn(&mut app);
    assert!(frame.contains("←→ year  {} marked year"), "{frame}");
    press_key(&mut app, KeyCode::Char('t'));
    press_key(&mut app, KeyCode::Right);
    assert_eq!(ledger_year(&mut app), TODAY.0 + 1, "and in the table");
    press_key(&mut app, KeyCode::Char('n'));
    assert_eq!(
        ledger_year(&mut app),
        TODAY.0 + 1,
        "the basis keeps the year"
    );
    assert!(redrawn(&mut app).contains("· future dollars ·"));
}

#[test]
fn the_braces_step_to_the_nearest_marked_year_and_say_where_there_is_none() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::Char('{'));
    assert_eq!(cursor(&app), None, "nothing is marked before the first");
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some("No marked year before 2026")
    );
    press_key(&mut app, KeyCode::Char('}'));
    assert_eq!(cursor(&app), Some(2032), "the inheritance");
    let frame = redrawn(&mut app);
    assert!(frame.contains("▌ 2032  57/53  ◆"), "{frame}");
    assert!(frame.contains("◆ "), "the year says its milestone: {frame}");
    press_key(&mut app, KeyCode::Char('}'));
    assert_eq!(cursor(&app), Some(2037));
    press_key(&mut app, KeyCode::Char('{'));
    assert_eq!(cursor(&app), Some(2032));
    press_key(&mut app, KeyCode::End);
    press_key(&mut app, KeyCode::Char('}'));
    assert!(
        said(&app)
            .last()
            .unwrap()
            .starts_with("No marked year after "),
        "{:?}",
        said(&app)
    );
}

#[test]
fn enter_on_the_tax_pane_shows_the_tax_tables_at_the_year() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::Right);
    for _ in 0..6 {
        press_key(&mut app, KeyCode::Tab);
    }
    assert!(composed_frame(&app).contains("⏎ tax tables"));
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::TaxTables);
    let frame = redrawn(&mut app);
    assert!(frame.contains(&(TODAY.0 + 1).to_string()), "{frame}");
}

/// The row a pane titled `title` starts on, and the column.
fn corner_of(frame: &str, title: &str) -> (usize, usize) {
    let found = frame.lines().enumerate().find_map(|(row, line)| {
        let before = line.split_once(title)?.0;
        Some((row, before.chars().count()))
    });
    found.unwrap_or_else(|| panic!("no {title}: {frame}"))
}

/// The rows of the page between the tab row and the key row.
fn page_rows(size: TerminalSize) -> usize {
    usize::from(size.rows - crate::layout::CHROME_ROWS)
}

#[test]
fn a_landscape_page_stacks_the_money_beside_what_the_year_does() {
    let mut app = ledger();
    let frame = redrawn(&mut app);
    let (table, _) = corner_of(&frame, "╭ Ledger ");
    let (to_do, _) = corner_of(&frame, "╭ To do in ");
    assert_eq!(to_do - table, page_rows(ROOMY) / 3, "a third: {frame}");
    let (flows, flows_at) = corner_of(&frame, "╭ Flows ");
    let money = ["╭ Money in ", "╭ Money out ", "╭ Tax "].map(|title| corner_of(&frame, title));
    assert_eq!(money[0].0, to_do, "beside the year's first row: {frame}");
    assert!(flows > to_do && flows_at == 0, "{frame}");
    assert!(
        money[0].0 < money[1].0 && money[1].0 < money[2].0,
        "one over the next: {frame}"
    );
    assert!(
        money
            .iter()
            .all(|&(_, at)| at == money[0].1 && at > flows_at),
        "in one column: {frame}"
    );
    let foot = frame.lines().nth(frame.lines().count() - 2).unwrap();
    assert_eq!(
        foot.matches('╰').count(),
        2,
        "the flows and the tax run to the page's foot: {frame}"
    );
    assert!(frame.contains("To top of 12% "), "{frame}");
}

#[test]
fn a_portrait_page_rows_the_money_under_flows_as_tall_as_their_rows() {
    let mut app = ledger_at(TALL);
    let frame = redrawn(&mut app);
    let (table, _) = corner_of(&frame, "╭ Ledger ");
    let (to_do, _) = corner_of(&frame, "╭ To do in ");
    assert_eq!(to_do - table, page_rows(TALL) / 2, "a half: {frame}");
    let (flows, _) = corner_of(&frame, "╭ Flows ");
    let money = ["╭ Money in ", "╭ Money out ", "╭ Tax "].map(|title| corner_of(&frame, title));
    assert!(
        money.iter().all(|&(row, _)| row == money[0].0),
        "side by side: {frame}"
    );
    let accounts = 6;
    let further_moves = 1;
    let header_total_and_borders = 4;
    assert_eq!(
        money[0].0 - flows,
        accounts + further_moves + header_total_and_borders,
        "{frame}"
    );
    let lines: Vec<&str> = frame.lines().collect();
    assert!(lines[money[0].0 - 2].contains("All accounts"), "{frame}");
    assert!(
        frame.contains("+$15,000 · 5.0%"),
        "growth beside its rate: {frame}"
    );
}

#[test]
fn the_headers_stay_over_the_years_as_they_scroll() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::End);
    let last = ledger_year(&mut app);
    let frame = redrawn(&mut app);
    let (table, _) = corner_of(&frame, "╭ Ledger ");
    let lines: Vec<&str> = frame.lines().collect();
    let heads = lines[table + 1].trim_end_matches('│').trim_end();
    assert!(
        heads.contains("Year  Age") && heads.ends_with("Net worth"),
        "{frame}"
    );
    let (year, _) = corner_of(&frame, &format!("▌ {last} "));
    assert!(year > table + 1, "the last year is scrolled to: {frame}");
    let scrollbar_and_border = 2;
    assert_eq!(
        heads.chars().count(),
        lines[year].chars().count() - scrollbar_and_border,
        "the net worth stands under its header: {frame}"
    );
}

#[test]
fn a_terminal_resized_between_the_shapes_is_laid_out_again() {
    let mut app = ledger();
    for (size, is_beside) in [(TALL, false), (ROOMY, true)] {
        app.insert_resource(size);
        for _ in 0..SETTLING_TICKS {
            app.update();
        }
        let frame = redrawn(&mut app);
        let (to_do, _) = corner_of(&frame, "╭ To do in ");
        let (money, _) = corner_of(&frame, "╭ Money in ");
        assert_eq!(money == to_do, is_beside, "{frame}");
    }
}

#[test]
fn a_terminal_too_short_for_the_year_opens_on_the_table_alone() {
    let mut app = ledger_at(SIZE);
    assert_eq!(view(&app), LedgerView::Table);
    let frame = redrawn(&mut app);
    assert!(!frame.contains("╭ Flows "), "{frame}");
    assert!(frame.contains("c columns  t the year"), "{frame}");
    press_key(&mut app, KeyCode::Char('t'));
    assert_eq!(view(&app), LedgerView::Year, "t still shows the year");
    assert!(redrawn(&mut app).contains("╭ Flows "));
    for (size, opened) in [(ROOMY, LedgerView::Year), (SIZE, LedgerView::Table)] {
        app.insert_resource(size);
        app.update();
        assert_eq!(view(&app), opened, "{size:?}");
    }
}

#[test]
fn a_year_that_is_shown_again_says_what_moved_while_it_was_hidden() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::Char('t'));
    for _ in 0..4 {
        press_key(&mut app, KeyCode::Down);
    }
    press_key(&mut app, KeyCode::Char('n'));
    press_key(&mut app, KeyCode::Char('t'));
    let frame = redrawn(&mut app);
    let year = TODAY.0 + 4;
    assert!(frame.contains(&format!("╭ To do in {year} ")), "{frame}");
    assert!(frame.contains("╭ So far · future dollars "), "{frame}");
    show(&mut app, Page::Overview);
    press_key(&mut app, KeyCode::Char('n'));
    show(&mut app, Page::Ledger);
    let frame = redrawn(&mut app);
    assert!(frame.contains("· today's dollars "), "{frame}");
    assert!(frame.contains(&format!("▌ {year} ")), "{frame}");
}

#[test]
fn a_run_opened_while_the_table_is_shown_lands_on_its_year() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::Char('t'));
    assert_eq!(view(&app), LedgerView::Table);
    let run = app.world().resource::<Projected>().clone();
    app.world_mut().resource_mut::<LedgerRun>().0 = Some(("a run".to_owned(), run));
    app.update();
    assert_eq!(view(&app), LedgerView::Year);
    let frame = redrawn(&mut app);
    assert!(frame.contains("╭ To do in 2026 · a run ─"), "{frame}");
    assert!(
        frame.contains("esc the plan"),
        "the key row says how to leave: {frame}"
    );
    assert!(
        !frame.contains("To top of"),
        "a run has no bracket of the plan's: {frame}"
    );
    press_key(&mut app, KeyCode::Esc);
    assert!(redrawn(&mut app).contains("To top of"), "the plan has");
}

#[test]
fn the_terminal_s_own_theme_bands_the_tables_for_the_screen_it_is_on() {
    let mut app = ledger();
    let worn = app.world().resource::<Theme>().clone();
    assert_eq!(worn, Theme::terminal_on(Variant::Dark));
    let mut stripes = app
        .world_mut()
        .query_filtered::<&TableStripe, With<LedgerTable>>();
    let stripe = stripes.single(app.world()).unwrap().0;
    assert!(
        stripe.bg.is_some() && stripe.bg == worn.stripe,
        "{stripe:?}"
    );
}

/// The row a pane titled `title` starts on.
fn top_of(frame: &str, title: &str) -> Option<usize> {
    frame.lines().position(|line| line.contains(title))
}

#[test]
fn how_far_the_plan_has_come_is_a_pane_of_its_own_beside_what_to_do() {
    let mut app = ledger();
    for _ in 0..16 {
        press_key(&mut app, KeyCode::Down);
    }
    let frame = redrawn(&mut app);
    let lines: Vec<&str> = frame.lines().collect();
    let top = top_of(&frame, "╭ To do in 2042 ").expect("the year's to-dos");
    assert!(
        lines[top].contains("╭ So far · today's dollars "),
        "side by side: {frame}"
    );
    let said: Vec<&str> = lines[top + 1..top + 4].to_vec();
    for (line, wanted) in said.iter().zip(["Taxes", "Converted", "Withdrawn"]) {
        assert!(
            line.contains(wanted) && line.contains(" of $"),
            "{wanted}: {frame}"
        );
    }
    assert!(
        lines[top + 4].contains("╰") && !frame.contains("So far:"),
        "{frame}"
    );
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Tab);
    assert!(
        composed_frame(&app).contains("↑↓ line"),
        "⇥ reaches it after the to-dos"
    );
}
