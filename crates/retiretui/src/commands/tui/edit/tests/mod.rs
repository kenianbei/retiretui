//! Headless editing tests: the helpers every screen shares, and the
//! table view's own behaviour.

mod applies;
mod contributions;
mod details;
mod fields;
mod help;
mod history;
mod identity;
mod inputs;
mod invested;
mod keyboard;
mod leaving;
mod panes;
mod places;
mod scroll;
mod sorting;
mod structured;
mod triggers;

use bevy_app::App;
use bevy_ecs::prelude::Entity;
use bevy_input_focus::InputFocus;
use plurimus::term::KeyCode;
use plurimus::widgets::ActiveDescendant;
use retiretui_engine::plan::Plan;

use super::build::FormField;
use super::editing::EditSession;
use super::table::Row;
use super::{Draft, SCREENS};
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{
    self, Headless, ROOMY, SIZE, cell_fg, cell_of, composed_frame, headless_app, headless_app_at,
    press_key, show, type_text,
};
use crate::commands::tui::theme::Theme;

pub(super) fn fixture_app() -> Headless {
    fixture_app_sized(SIZE)
}

pub(crate) fn fixture_app_sized(size: plurimus::core::TerminalSize) -> Headless {
    headless_app_at(support::scratch_full_plan(), size)
}

pub(super) fn draft_plan(app: &App) -> Plan {
    app.world().resource::<Draft>().plan.clone()
}

pub(super) fn is_editing(app: &App) -> bool {
    app.world().resource::<EditSession>().is_open()
}

pub(super) fn focused(app: &App) -> Entity {
    app.world().resource::<InputFocus>().get().unwrap()
}

/// Whether the keyboard is on a `Pane`.
pub(super) fn is_on<Pane: bevy_ecs::component::Component>(app: &App) -> bool {
    app.world().get::<Pane>(focused(app)).is_some()
}

/// The row under the focused table's cursor.
pub(crate) fn cursor(app: &mut App) -> Row {
    let table = focused(app);
    let row = app
        .world()
        .get::<ActiveDescendant>(table)
        .unwrap()
        .0
        .unwrap();
    *app.world().get::<Row>(row).unwrap()
}

/// Backspaces the focused field empty so typing replaces its value.
pub(crate) fn clear_field(app: &mut App) {
    press_key(app, KeyCode::End);
    for _ in 0..40 {
        press_key(app, KeyCode::Backspace);
    }
}

/// How many stops along an account's form its balance is.
pub(crate) const BALANCE_FIELD: usize = 4;

/// Types a new balance into the open account, leaving the keyboard on the
/// field.
pub(crate) fn retype_balance(app: &mut App) {
    tab_to_field(app, BALANCE_FIELD);
    clear_field(app);
    type_text(app, "123456");
}

/// Opens the fixture's second expense, whose start follows an event and
/// whose end is an age.
pub(super) fn open_travel(app: &mut App) {
    show(app, Page::Expenses);
    app.update();
    press_key(app, KeyCode::Down);
    press_key(app, KeyCode::Enter);
}

/// Shows `page` and opens the item under its cursor: a table's first
/// row, or the one item a page without a table has.
pub(crate) fn open(app: &mut App, page: Page) {
    show(app, page);
    app.update();
    press_key(app, KeyCode::Enter);
}

/// Stops along a brokerage account's form to the pick that says how it is
/// invested.
pub(super) const INVESTED: usize = 6;

/// Where the open item's form stands: its top line, and the column of its
/// left border in cells.
pub(super) fn form_box(frame: &str) -> (usize, usize) {
    let found = frame.lines().enumerate().find_map(|(top, line)| {
        let (before, _) = line.split_once("╭ Edit")?;
        Some((top, before.chars().count()))
    });
    found.unwrap_or_else(|| panic!("no form: {frame}"))
}

/// Whether a row labelled exactly `label` is drawn: the label, then its
/// widget - a bracket or a slider - and nothing else between.
pub(super) fn shows_row(frame: &str, label: &str) -> bool {
    let is_row = |cell: &str| {
        cell.strip_prefix(label).is_some_and(|rest| {
            rest.starts_with(' ') && rest.trim_start().starts_with(['[', '█', '━', '─'])
        })
    };
    frame.lines().any(|line| line.split('│').any(is_row))
}

/// Tabs round the open form until the keyboard is in the field `key`.
pub(super) fn tab_to_key(app: &mut App, key: &str) {
    fields::tab_until(app, key, |app| {
        let field = app.world().get::<FormField>(focused(app));
        field.is_some_and(|field| field.spec.key == key)
    });
}

pub(crate) fn tab_to_field(app: &mut App, row: usize) {
    for _ in 0..row {
        press_key(app, KeyCode::Tab);
    }
}
#[test]
fn every_editing_page_renders_its_table() {
    // Wide enough that no cell is cut: beside the sidebar, eighty columns
    // clip the longest of the expenses' triggers.
    let mut app = fixture_app_sized(ROOMY);
    let expectations = [
        (
            Page::Accounts,
            vec![
                "Account",
                "Balance",
                "▌ cash",
                "fid-401k",
                "$450,000",
                ADD_ACCOUNT,
            ],
        ),
        (Page::Income, vec!["salary", "Social Security", "Amount"]),
        (
            Page::Expenses,
            vec![
                "living",
                "$90,000",
                "Growth",
                "1 yr after retire",
                "age 80 (jordan)",
            ],
        ),
        (Page::Transfers, vec!["pension-dc", "rollover-ira"]),
        (Page::Conversions, vec!["fid-401k"]),
        (Page::People, vec!["jordan", "1975-06-14"]),
        (Page::Settings, vec!["Plan to age", "95"]),
    ];
    for (page, expected) in expectations {
        show(&mut app, page);
        app.update();
        let frame = composed_frame(&app);
        for text in expected {
            assert!(frame.contains(text), "{page:?} lacks {text:?}: {frame}");
        }
    }
}

/// What the accounts table's add button says.
pub(crate) const ADD_ACCOUNT: &str = "+ Add Account";

/// A name longer than any width the expenses table was ever given, and
/// longer than every other cell in its column.
const LONG_NAME: &str = "housing-rental-year";

#[test]
fn a_name_longer_than_its_column_widens_it_rather_than_being_cut() {
    let path = support::scratch_full_plan();
    let plan = std::fs::read_to_string(&path)
        .expect("the copied fixture")
        .replace("id = \"living\"", &format!("id = \"{LONG_NAME}\""));
    std::fs::write(&path, plan).expect("the widened fixture");
    let mut app = headless_app_at(path, SIZE);
    show(&mut app, Page::Expenses);
    app.update();
    let frame = composed_frame(&app);
    assert!(
        frame.contains(LONG_NAME),
        "the narrowest table still draws it whole: {frame}"
    );
}

#[test]
fn the_cursor_moves_within_a_table() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    assert_eq!(cursor(&mut app), Row(0));
    press_key(&mut app, KeyCode::Down);
    assert_eq!(cursor(&mut app), Row(1));
    press_key(&mut app, KeyCode::End);
    let last = draft_plan(&app).accounts.len() - 1;
    assert_eq!(
        cursor(&mut app),
        Row(last),
        "the end of a table is its last item"
    );
}

#[test]
fn the_add_button_and_the_add_command_both_append() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    app.update();
    let (x, y) = cell_of(&app, ADD_ACCOUNT);
    support::click(&mut app, x + 1, y);
    assert!(is_editing(&app), "the button opens a new item");
    assert_eq!(
        draft_plan(&app).accounts.len(),
        2,
        "nothing added until apply"
    );
    clear_field(&mut app);
    type_text(&mut app, "brokerage");
    press_key(&mut app, KeyCode::Enter);
    let plan = draft_plan(&app);
    assert_eq!(plan.accounts.len(), 3);
    assert_eq!(plan.accounts[2].id, "brokerage");
    assert_eq!(plan.accounts[2].owner, "me");
    app.update();
    assert_eq!(cursor(&mut app), Row(2));
    press_key(&mut app, KeyCode::Char('a'));
    assert!(is_editing(&app), "the add command opens one too");
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(draft_plan(&app).accounts.len(), 3);
}

#[test]
fn delete_asks_before_it_removes_the_item() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Char('d'));
    let frame = composed_frame(&app);
    assert!(frame.contains("Delete cash?"), "{frame}");
    assert!(
        frame.contains("[ Delete ]"),
        "the button says what it does: {frame}"
    );
    let (column, row) = cell_of(&app, "[ Delete ]");
    let over = app.world().resource::<Theme>().over;
    assert_eq!(cell_fg(&app, column, row), Some(over), "and that it loses");
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(draft_plan(&app).accounts.len(), 2, "esc leaves it undone");
    press_key(&mut app, KeyCode::Down);
    assert_eq!(
        cursor(&mut app),
        Row(1),
        "and gives the table the keyboard back"
    );

    press_key(&mut app, KeyCode::Char('d'));
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(
        draft_plan(&app).accounts.len(),
        2,
        "cancel leaves it undone"
    );

    press_key(&mut app, KeyCode::Char('d'));
    press_key(&mut app, KeyCode::Enter);
    let accounts = draft_plan(&app).accounts;
    assert_eq!(
        accounts.len(),
        1,
        "the keyboard opens on the answer asked for"
    );
    assert_eq!(
        accounts[0].id, "cash",
        "the highlighted item is the one removed"
    );
}

#[test]
fn a_domain_emptied_of_items_says_what_it_is_for() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    for _ in 0..draft_plan(&app).accounts.len() {
        press_key(&mut app, KeyCode::Char('d'));
        press_key(&mut app, KeyCode::Enter);
    }
    app.update();
    assert!(draft_plan(&app).accounts.is_empty());
    let frame = composed_frame(&app);
    assert!(
        frame.contains(<super::accounts::Accounts as super::domain::Domain>::PURPOSE),
        "an empty domain says what it is for: {frame}"
    );
    press_key(&mut app, KeyCode::Char('d'));
    let frame = composed_frame(&app);
    assert!(
        frame.contains("nothing highlighted"),
        "and holds no item to delete: {frame}"
    );
}

#[test]
fn scenario_sessions_refuse_to_edit() {
    let scenario = support::scratch_scenario();
    let mut app = headless_app_at(scenario, SIZE);
    open(&mut app, Page::Accounts);
    assert!(!is_editing(&app));
    let frame = composed_frame(&app);
    assert!(frame.contains("read-only"), "{frame}");
    assert!(
        !frame.contains(ADD_ACCOUNT),
        "nothing offers what the session refuses: {frame}"
    );
}

#[test]
fn every_domain_declares_columns_that_are_fields_and_a_usable_blank() {
    let mut draft = Draft::new(support::test_projected().plan, false);
    for ops in SCREENS {
        let Some(list) = ops.list else { continue };
        for column in list.columns {
            assert!(
                column.phrase.is_some() || ops.fields.iter().any(|field| field.key == column.key),
                "{}: column {}",
                ops.title,
                column.key
            );
        }
        let before = (list.rows)(&draft.plan).len();
        let blank = (list.blank)(&draft.plan);
        (ops.store)(&mut draft, before, blank)
            .unwrap_or_else(|error| panic!("{}: {error}", ops.title));
        assert_eq!((list.rows)(&draft.plan).len(), before + 1, "{}", ops.title);
        (list.remove)(&mut draft.plan, before);
        assert_eq!((list.rows)(&draft.plan).len(), before, "{}", ops.title);
    }
}

/// An item opened and applied untouched must be the item it was, so every
/// value the full fixture holds has to survive the text its field shows.
#[test]
fn every_text_field_reads_back_the_value_it_shows() {
    use super::cells::{field_text, parse_field};
    use super::codec::get_path;
    use super::domain::FieldKind;

    let plan: Plan = toml::from_str(include_str!(
        "../../../../../../retiretui_engine/tests/fixtures/full.toml"
    ))
    .unwrap();
    let draft = Draft::new(plan, false);
    let mut checked = 0;
    for ops in SCREENS {
        let count = ops.list.map_or(1, |list| (list.count)(&draft.plan));
        for index in 0..count {
            let item = (ops.item)(&draft, index).expect("an item the domain counted");
            for spec in ops.fields {
                let is_typed = matches!(
                    spec.kind,
                    FieldKind::Text
                        | FieldKind::Money
                        | FieldKind::Whole
                        | FieldKind::Rate
                        | FieldKind::Growth
                );
                let Some(value) = get_path(&item, spec.key).filter(|_| is_typed) else {
                    continue;
                };
                for is_focused in [false, true] {
                    let shown = field_text(spec.kind, Some(value), is_focused);
                    let read = parse_field(spec.kind, &shown);
                    assert_eq!(
                        read.as_ref(),
                        Some(value),
                        "{} {}: {shown}",
                        ops.title,
                        spec.key
                    );
                }
                checked += 1;
            }
        }
    }
    assert!(
        checked > 40,
        "the fixture fills fields of every kind: {checked}"
    );
}

#[test]
fn an_issue_reads_in_the_forms_words_and_an_unknown_path_as_written() {
    use retiretui_engine::plan::Issue;

    let draft = Draft::new(support::test_projected().plan, false);
    let issue = |path: &str| Issue {
        path: path.to_owned(),
        message: "is wrong".to_owned(),
    };
    let cases = [
        ("plan.inflation", "Settings \u{203a} Inflation: is wrong"),
        (
            "household.filing",
            "Household \u{203a} Filing status: is wrong",
        ),
        (
            "medicare.prior_magi",
            "Household \u{203a} Income last year: is wrong",
        ),
        (
            "medicare.part_d",
            "Household \u{203a} Include Part D: is wrong",
        ),
        (
            "medicare",
            "Household \u{203a} Medicare surcharges: is wrong",
        ),
        ("accounts", "Accounts: is wrong"),
        (
            "accounts[99].balance",
            "Accounts \u{203a} Balance: is wrong",
        ),
        (
            "accounts[99].allocation",
            "Accounts \u{203a} Stocks: is wrong",
        ),
        (
            "accounts[99].allocation[1]",
            "Accounts \u{203a} Mix 2 from: is wrong",
        ),
        (
            "contributions[99].match.up_to",
            "Contributions \u{203a} Matched up to: is wrong",
        ),
        ("schema", "schema: is wrong"),
    ];
    for (path, words) in cases {
        assert_eq!(super::issue_words(&issue(path), &draft), words, "{path}");
    }
    let named = super::issue_words(&issue("household.people[0].birth"), &draft);
    assert!(named.starts_with("People \u{203a} "), "{named}");
    assert!(named.ends_with(" \u{203a} Birth date: is wrong"), "{named}");
}

const STATEMENT: &str = "../retiretui_engine/tests/fixtures/statement.xml";

#[test]
fn a_statement_picked_on_the_people_page_is_recorded_on_the_highlighted_person() {
    let statement = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(STATEMENT);
    let statement = statement.to_str().unwrap();
    let mut app = fixture_app();
    show(&mut app, Page::People);
    press_key(&mut app, KeyCode::Char('e'));
    let frame = composed_frame(&app);
    assert!(frame.contains("Import earnings from"), "{frame}");
    type_text(&mut app, statement);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let plan = draft_plan(&app);
    assert_eq!(plan.household.people[0].earnings.len(), 3, "{plan:?}");
    assert!(app.world().resource::<Draft>().is_dirty());
    let frame = composed_frame(&app);
    assert!(frame.contains("1995-2024"), "{frame}");
    // The second person was born on another date, so the same statement
    // is refused and nothing changes.
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('e'));
    type_text(&mut app, statement);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert!(draft_plan(&app).household.people[1].earnings.is_empty());
}
