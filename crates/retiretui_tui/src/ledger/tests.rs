//! The Ledger as a user drives it: its two views, the table's column
//! sets, and the year stepped from any pane.

use bevy_app::App;
use bevy_input_focus::InputFocus;
use plurimus::term::KeyCode;

use super::{Columns, LedgerTable, LedgerView};
use crate::nav::Page;
use crate::session::{LedgerRun, Projected, YearCursor};
use crate::support::{
    SIZE, TODAY, active_page, composed_frame, headless_app_at, ledger_year, press_key, redrawn,
    said, scratch_full_plan, show,
};
use retiretui_client::ledger::ColumnSet;

/// The full fixture on the Ledger: several accounts, milestones and a
/// ladder of conversions.
fn ledger() -> crate::support::Headless {
    let mut app = headless_app_at(scratch_full_plan(), SIZE);
    show(&mut app, Page::Ledger);
    app
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
fn t_swaps_the_page_for_the_table_and_back_at_the_same_year() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Tab);
    assert!(!holds_the_table(&app), "the flows hold the keyboard");
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
    assert!(holds_the_table(&app), "which is the list of years again");
    let frame = redrawn(&mut app);
    assert!(frame.contains("╭ Years "), "{frame}");
    assert!(frame.contains(&format!("╭ {} · ", TODAY.0 + 2)), "{frame}");
    assert!(frame.contains("t table"), "{frame}");
}

#[test]
fn enter_in_the_table_shows_the_year_its_cursor_is_on() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(view(&app), LedgerView::Year, "⏎ on the list keeps the year");
    press_key(&mut app, KeyCode::Char('t'));
    for _ in 0..3 {
        press_key(&mut app, KeyCode::Down);
    }
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(view(&app), LedgerView::Year);
    let frame = redrawn(&mut app);
    assert!(frame.contains(&format!("╭ {} · ", TODAY.0 + 3)), "{frame}");
}

#[test]
fn c_turns_the_table_through_its_column_sets() {
    let mut app = ledger();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('c'));
    assert_eq!(view(&app), LedgerView::Table, "c shows the table it turns");
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
        assert_eq!(ledger_year(&mut app), TODAY.0 + 1, "the year stays");
        press_key(&mut app, KeyCode::Char('c'));
    }
}

#[test]
fn the_year_is_stepped_from_any_pane_by_the_arrows_or_the_brackets() {
    let mut app = ledger();
    for _ in 0..2 {
        press_key(&mut app, KeyCode::Tab);
    }
    assert!(!holds_the_table(&app));
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
    for _ in 0..5 {
        press_key(&mut app, KeyCode::Tab);
    }
    assert!(composed_frame(&app).contains("⏎ tax tables"));
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::TaxTables);
    let frame = redrawn(&mut app);
    assert!(frame.contains(&(TODAY.0 + 1).to_string()), "{frame}");
}

#[test]
fn the_flows_are_as_tall_as_their_rows_and_say_every_account_as_one() {
    let mut app = ledger();
    let frame = redrawn(&mut app);
    let lines: Vec<&str> = frame.lines().collect();
    let top = lines.iter().position(|line| line.contains("╭ Flows "));
    let under = lines.iter().position(|line| line.contains("╭ Money in "));
    let (top, under) = (top.unwrap(), under.unwrap());
    let accounts = 6;
    let further_moves = 1;
    let header_total_and_borders = 4;
    assert_eq!(
        under - top,
        accounts + further_moves + header_total_and_borders,
        "{frame}"
    );
    assert!(lines[under - 2].contains("All accounts"), "{frame}");
    assert!(
        frame.contains("+$15,000 · 5.0%"),
        "growth beside its rate: {frame}"
    );
    assert!(frame.contains("╭ Money out "), "{frame}");
    assert!(frame.contains("To top of 12% "), "{frame}");
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
    assert!(
        frame.contains(&format!(
            "╭ {year} · jordan turns 55 · alex turns 51 · future dollars "
        )),
        "{frame}"
    );
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
    assert!(
        frame.contains("╭ 2026 · a run · esc returns to the plan · "),
        "{frame}"
    );
    assert!(
        !frame.contains("To top of"),
        "a run has no bracket of the plan's: {frame}"
    );
    press_key(&mut app, KeyCode::Esc);
    assert!(redrawn(&mut app).contains("To top of"), "the plan has");
}
