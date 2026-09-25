//! Headless tests for a table shown in an order other than the plan's.

use bevy_app::App;
use plurimus::term::KeyCode;

use super::{
    ADD_ACCOUNT, BALANCE_FIELD, clear_field, cursor, draft_plan, fixture_app, tab_to_field,
};
use crate::commands::tui::support::cell_of;

/// What heads the accounts table's balance column.
const BALANCE_HEADER: &str = "Balance";
use crate::commands::tui::edit::table::Row;
use crate::commands::tui::layout;
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{click, composed_frame, press_key, show};

/// Presses the header of the balance column, wherever the table drew it.
fn click_balance(app: &mut App) {
    let (x, y) = cell_of(app, BALANCE_HEADER);
    click(app, x, y);
}

/// The border a pane draws down its right-hand side, which is all an
/// empty row below the last item holds.
const PANE_EDGE: &str = "│";

/// Where the sidebar's pane ends and the table's begins.
const PANE_JOIN: &str = "││";

/// The ids the accounts table lists, top to bottom.
fn listed(app: &App) -> Vec<String> {
    let frame = composed_frame(app);
    let rows = frame
        .lines()
        .skip(usize::from(layout::BODY_TOP) + 2)
        .filter_map(|line| line.split_once(PANE_JOIN))
        .map(|(_, table)| table.trim_start_matches(['▌', '▏']))
        .map(|table| table.split_whitespace().next().unwrap_or_default());
    rows.take_while(|id| !id.is_empty() && *id != PANE_EDGE)
        .map(str::to_owned)
        .collect()
}

fn accounts_app() -> crate::commands::tui::support::Headless {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    app
}

#[test]
fn a_header_orders_the_table_up_then_down_then_as_the_plan_has_it() {
    let mut app = accounts_app();
    let planned = listed(&app);
    assert_eq!(planned[0], "cash");
    click_balance(&mut app);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("Balance ▲"), "{frame}");
    assert_eq!(
        listed(&app)[0],
        "rollover-ira",
        "a balance of nothing: {frame}"
    );
    assert_eq!(listed(&app).last().map(String::as_str), Some("fid-401k"));
    assert!(
        frame.contains(ADD_ACCOUNT),
        "the button stays under the rows"
    );
    click_balance(&mut app);
    app.update();
    assert!(composed_frame(&app).contains("Balance ▼"));
    assert_eq!(listed(&app)[0], "fid-401k");
    click_balance(&mut app);
    app.update();
    assert_eq!(listed(&app), planned);
}

#[test]
fn the_sort_key_walks_the_columns() {
    let mut app = accounts_app();
    press_key(&mut app, KeyCode::Char('s'));
    assert!(composed_frame(&app).contains("Account ▲"));
    assert_eq!(listed(&app)[0], "brokerage");
    press_key(&mut app, KeyCode::Char('s'));
    assert!(composed_frame(&app).contains("Account ▼"));
    press_key(&mut app, KeyCode::Char('s'));
    assert!(composed_frame(&app).contains("Type ▲"));
}

#[test]
fn an_edit_under_a_sort_lands_in_the_item_the_row_stands_for() {
    let mut app = accounts_app();
    press_key(&mut app, KeyCode::Char('s'));
    assert_eq!(cursor(&mut app), Row(0), "the cursor stays on cash");
    press_key(&mut app, KeyCode::Home);
    assert_eq!(cursor(&mut app), Row(1), "brokerage leads by id");
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, BALANCE_FIELD);
    clear_field(&mut app);
    crate::commands::tui::support::type_text(&mut app, "123456");
    press_key(&mut app, KeyCode::Enter);
    let accounts = draft_plan(&app).accounts;
    assert_eq!(accounts[1].id, "brokerage", "the plan keeps its own order");
    assert_eq!(accounts[1].balance, 123_456);
    assert_eq!(cursor(&mut app), Row(1));
}
