use bevy_app::App;
use bevy_input_focus::InputFocus;
use plurimus::term::KeyCode;

use super::Sidebar;
use crate::commands::tui::edit::DomainTable;
use crate::commands::tui::edit::tests::open;
use crate::commands::tui::nav::{FocusStop, Group, Page, tab_digit};
use crate::commands::tui::support::{
    SIZE, active_page, cell_of, click, commit_edit, composed_frame, headless_app, press_key,
    press_shift, redrawn, run_command, show,
};

/// The bar of the list the keyboard is in.
const HELD_BAR: char = '▌';

fn is_on_sidebar(app: &App) -> bool {
    let focused = app.world().resource::<InputFocus>().get();
    focused.is_some_and(|entity| app.world().get::<Sidebar>(entity).is_some())
}

/// Whether the keyboard is on the page's own holder, which for a domain
/// that is a form alone is the form.
fn is_on_form(app: &App) -> bool {
    let focused = app.world().resource::<InputFocus>().get();
    focused.is_some_and(|entity| app.world().get::<FocusStop>(entity).is_some())
}

fn is_on_table(app: &App) -> bool {
    let focused = app.world().resource::<InputFocus>().get();
    focused.is_some_and(|entity| app.world().get::<DomainTable>(entity).is_some())
}

/// The sidebar's row for `page`, as drawn.
fn sidebar_row(app: &App, page: Page) -> String {
    let frame = composed_frame(app);
    let row = frame.lines().find(|line| {
        line.trim_start_matches(['│', HELD_BAR, '▏', ' '])
            .starts_with(page.title())
    });
    row.unwrap_or_else(|| panic!("{page:?}: {frame}"))
        .to_owned()
}

#[test]
fn the_sidebar_lists_every_domain_beside_the_one_on_show() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char(tab_digit(Group::Plan.tab())));
    let frame = redrawn(&mut app);
    for page in Page::ALL.into_iter().filter(|page| page.is_domain()) {
        assert!(frame.contains(page.title()), "{page:?}: {frame}");
    }
    assert!(frame.contains("╭ Plan ") && frame.contains("╭ Accounts "));
    show(&mut app, Page::Ledger);
    assert!(
        !redrawn(&mut app).contains("╭ Plan "),
        "and beside nothing else"
    );
}

#[test]
fn the_tab_lands_on_the_sidebar_over_the_domain_last_shown() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char(tab_digit(Group::Plan.tab())));
    assert_eq!(active_page(&app), Page::Accounts, "the first of them");
    assert!(is_on_sidebar(&app));

    show(&mut app, Page::Expenses);
    assert!(is_on_table(&app), "a domain's own command goes into it");
    press_key(&mut app, KeyCode::Char(tab_digit(0)));
    press_key(&mut app, KeyCode::Char(tab_digit(Group::Plan.tab())));
    assert_eq!(active_page(&app), Page::Expenses);
    assert!(is_on_sidebar(&app), "and the tab's digit does not");
    assert!(sidebar_row(&app, Page::Expenses).contains(HELD_BAR));
}

#[test]
fn the_tools_tab_lands_on_its_sidebar_over_the_tool_last_shown() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char(tab_digit(Group::Tools.tab())));
    assert_eq!(
        active_page(&app),
        Page::RothConversions,
        "the first of them"
    );
    assert!(is_on_sidebar(&app));
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("╭ Tools ") && frame.contains("↑↓ tool") && !frame.contains("╭ Plan "),
        "{frame}"
    );
    assert!(sidebar_row(&app, Page::RothConversions).contains(HELD_BAR));

    press_key(&mut app, KeyCode::Right);
    assert!(is_on_form(&app), "the arrow goes into the tool");
    press_key(&mut app, KeyCode::Esc);
    assert!(is_on_sidebar(&app), "and esc comes back");
    assert_eq!(active_page(&app), Page::RothConversions);

    run_command(&mut app, "ledger");
    run_command(&mut app, "roth-conversions");
    assert!(is_on_form(&app), "a tool's own command goes into it");
    run_command(&mut app, "tab-previous");
    assert_eq!(active_page(&app), Page::Compare);
    run_command(&mut app, "tab-next");
    assert_eq!(
        active_page(&app),
        Page::RothConversions,
        "the tab remembers"
    );
    run_command(&mut app, "tab-next");
    assert_eq!(
        active_page(&app),
        Page::Accounts,
        "and the plan's tab is next"
    );
}

#[test]
fn the_cursor_is_the_page_whichever_of_them_moves() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char(tab_digit(Group::Plan.tab())));
    press_key(&mut app, KeyCode::Down);
    assert_eq!(
        active_page(&app),
        Page::Income,
        "the page follows the cursor"
    );
    assert!(is_on_sidebar(&app), "which keeps the keyboard");
    assert!(redrawn(&mut app).contains("╭ Income "));

    run_command(&mut app, "people");
    app.update();
    assert!(
        sidebar_row(&app, Page::People).contains('▏'),
        "and the cursor follows the page: {}",
        composed_frame(&app)
    );
}

#[test]
fn arrows_and_esc_move_between_the_sidebar_and_the_domain() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char(tab_digit(Group::Plan.tab())));
    assert!(redrawn(&mut app).contains("→ into"));
    press_key(&mut app, KeyCode::Right);
    assert!(is_on_table(&app));
    assert!(redrawn(&mut app).contains("← domains"));
    press_key(&mut app, KeyCode::Left);
    assert!(is_on_sidebar(&app));
    press_key(&mut app, KeyCode::Enter);
    assert!(is_on_table(&app));
    press_key(&mut app, KeyCode::Esc);
    assert!(is_on_sidebar(&app));
    assert_eq!(active_page(&app), Page::Accounts, "the page never turned");
}

#[test]
fn esc_on_a_page_that_is_not_a_domain_keeps_it() {
    let mut app = headless_app(SIZE);
    run_command(&mut app, "ledger");
    press_key(&mut app, KeyCode::Esc);
    press_key(&mut app, KeyCode::Left);
    assert_eq!(active_page(&app), Page::Ledger);
    assert!(!is_on_sidebar(&app));
}

#[test]
fn a_lone_form_is_never_a_dead_end() {
    for page in [Page::Household, Page::Settings] {
        let mut app = headless_app(SIZE);
        show(&mut app, page);
        assert!(redrawn(&mut app).contains("esc domains"), "{page:?}");
        press_key(&mut app, KeyCode::Esc);
        assert!(is_on_sidebar(&app), "{page:?} hands the keyboard back");
        press_key(&mut app, KeyCode::Up);
        assert_ne!(active_page(&app), page, "and another domain is a key away");

        open(&mut app, page);
        press_key(&mut app, KeyCode::Esc);
        press_key(&mut app, KeyCode::Esc);
        assert!(is_on_sidebar(&app), "{page:?}: out of a field, then out");
    }
}

#[test]
fn only_the_list_with_the_keyboard_draws_the_full_bar() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char(tab_digit(Group::Plan.tab())));
    let held = |frame: &str| frame.matches(HELD_BAR).count();
    let frame = redrawn(&mut app);
    assert_eq!(held(&frame), 1, "{frame}");
    assert!(sidebar_row(&app, Page::Accounts).contains(HELD_BAR));
    press_key(&mut app, KeyCode::Right);
    let frame = redrawn(&mut app);
    assert_eq!(held(&frame), 1, "{frame}");
    assert!(sidebar_row(&app, Page::Accounts).contains('▏'), "{frame}");
}

#[test]
fn the_counts_follow_the_draft() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    let before = sidebar_row(&app, Page::Accounts);
    commit_edit(&mut app, |plan| {
        let spare = plan.accounts[0].clone();
        plan.accounts.push(retiretui_engine::plan::Account {
            id: "spare".to_owned(),
            ..spare
        });
    });
    redrawn(&mut app);
    let after = sidebar_row(&app, Page::Accounts);
    assert!(before.contains(" 2 ") && after.contains(" 3 "), "{after}");
}

#[test]
fn a_lone_form_takes_the_keyboard_as_a_table_does() {
    for (page, label) in [
        (Page::Household, "Filing status"),
        (Page::Settings, "Plan name"),
    ] {
        let mut app = headless_app(SIZE);
        show(&mut app, page);
        press_key(&mut app, KeyCode::Esc);
        assert!(is_on_sidebar(&app));
        press_key(&mut app, KeyCode::Right);
        assert!(is_on_form(&app), "{page:?}: the arrow goes into it");

        press_key(&mut app, KeyCode::Esc);
        let (x, y) = cell_of(&app, label);
        click(&mut app, x, y);
        app.update();
        assert!(is_on_form(&app), "{page:?}: and so does a press on it");
    }
}

#[test]
fn tab_and_shift_tab_walk_between_the_sidebar_and_the_domain() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char(tab_digit(Group::Plan.tab())));
    assert!(
        redrawn(&mut app).contains("⇥ pane"),
        "the sidebar names the key"
    );
    press_key(&mut app, KeyCode::Tab);
    assert!(is_on_table(&app), "tab goes on to the domain");
    press_shift(&mut app, KeyCode::Tab);
    assert!(is_on_sidebar(&app), "shift-tab comes back");
    press_shift(&mut app, KeyCode::Tab);
    assert!(!is_on_sidebar(&app), "and wraps to the page's last pane");
    assert_eq!(active_page(&app), Page::Accounts, "the page never turned");
}

#[test]
fn tab_is_quiet_on_a_page_of_one_pane_or_none() {
    for command in ["ledger", "overview"] {
        let mut app = headless_app(SIZE);
        run_command(&mut app, command);
        let held = app.world().resource::<InputFocus>().get();
        press_key(&mut app, KeyCode::Tab);
        press_shift(&mut app, KeyCode::Tab);
        let holder = app.world().resource::<InputFocus>().get();
        assert_eq!(holder, held, "{command}");
    }
}

#[test]
fn a_press_on_the_tab_bar_turns_the_page_and_never_holds_the_keyboard() {
    let mut app = headless_app(SIZE);
    run_command(&mut app, "ledger");
    let (x, y) = cell_of(&app, "Compare");
    click(&mut app, x, y);
    app.update();
    assert_eq!(active_page(&app), Page::Compare);
    let focused = app.world().resource::<InputFocus>().get();
    assert!(is_on_form(&app), "the page shown has the keyboard");
    click(&mut app, x, y);
    app.update();
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        focused,
        "and keeps it"
    );
}
