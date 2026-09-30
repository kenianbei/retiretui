//! Headless tests for the details pane beside a table: what it shows of
//! the row under the cursor, and what ⏎ on it opens.

use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::{Entity, With};
use plurimus::core::ratatui_core::style::Modifier;
use plurimus::term::KeyCode;
use plurimus::ui::UiStyle;
use retiretui_client::forms::{DOMAINS, DomainId, details};

use super::{clear_field, cursor, draft_plan, fixture_app, fixture_app_sized, is_editing, is_on};
use crate::edit::details::DetailsTable;
use crate::edit::draft::Draft;
use crate::edit::table::{DomainTable, Row};
use crate::nav::Page;
use crate::present::money;
use crate::support::{SIZE, commit_edit, composed_frame, press_key, redrawn, show, type_text};

#[test]
fn the_details_show_the_row_under_the_cursor_in_the_forms_words() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    let frame = composed_frame(&app);
    assert!(frame.contains("╭ Account ─"), "{frame}");
    assert!(frame.contains("Balance"), "{frame}");
    let first = draft_plan(&app).accounts[0].balance;
    assert!(frame.contains(&money(first)), "{frame}");
    assert!(
        !frame.contains("Basis"),
        "a field the item has no use for is left out: {frame}"
    );
    press_key(&mut app, KeyCode::Down);
    let frame = redrawn(&mut app);
    assert!(frame.contains("$300,000"), "the next row: {frame}");
    assert!(frame.contains("Basis"), "and one it has is in: {frame}");
    assert!(is_on::<DomainTable>(&app), "looking at a row opens nothing");
}

#[test]
fn enter_on_the_details_opens_the_item_and_closing_gives_the_pane_back() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Tab);
    assert!(is_on::<DetailsTable>(&app));
    press_key(&mut app, KeyCode::Enter);
    assert!(is_editing(&app), "the item under the cursor opens");
    press_key(&mut app, KeyCode::Esc);
    assert!(is_on::<DetailsTable>(&app), "esc gives back what opened it");
    press_key(&mut app, KeyCode::Enter);
    press_key(&mut app, KeyCode::Enter);
    assert!(
        !is_editing(&app) && is_on::<DetailsTable>(&app),
        "and so does an apply"
    );
}

#[test]
fn a_persons_details_end_with_the_earnings_record_and_scroll() {
    let mut app = fixture_app();
    commit_edit(&mut app, |plan| {
        let person = &mut plan.household.people[0];
        for year in 1990..2026 {
            person
                .earnings
                .insert(year, 50_000 + i64::from(year - 1990) * 1_000);
        }
    });
    show(&mut app, Page::People);
    let frame = composed_frame(&app);
    assert!(frame.contains("Earnings"), "{frame}");
    assert!(
        frame.contains("1990") && frame.contains("$50,000"),
        "{frame}"
    );
    assert!(!frame.contains("$85,000"), "past the pane's foot: {frame}");
    press_key(&mut app, KeyCode::Tab);
    assert!(is_on::<DetailsTable>(&app));
    for _ in 0..40 {
        press_key(&mut app, KeyCode::Down);
    }
    let frame = redrawn(&mut app);
    assert!(frame.contains("$85,000"), "scrolled to the end: {frame}");
}

#[test]
fn only_the_table_on_show_is_followed() {
    let mut app = fixture_app_sized(SIZE);
    show(&mut app, Page::Income);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("╭ Income ─") && !frame.contains("╭ Account ─"),
        "{frame}"
    );
}

#[test]
fn a_domain_emptied_can_still_be_added_to() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    for _ in 0..draft_plan(&app).accounts.len() {
        press_key(&mut app, KeyCode::Char('d'));
        press_key(&mut app, KeyCode::Enter);
    }
    app.update();
    assert!(draft_plan(&app).accounts.is_empty());
    press_key(&mut app, KeyCode::Char('a'));
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("╭ New Account "), "{frame}");
    clear_field(&mut app);
    type_text(&mut app, "spare");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(
        draft_plan(&app)
            .accounts
            .first()
            .map(|held| held.id.as_str()),
        Some("spare"),
        "and applies"
    );
    assert_eq!(cursor(&mut app), Row(0));
}

#[test]
fn an_arrow_on_an_empty_domain_moves_no_cursor_onto_its_line() {
    let mut app = fixture_app();
    show(&mut app, Page::Cliffs);
    app.update();
    let said =
        <retiretui_client::forms::expenses::Cliffs as retiretui_client::forms::Domain>::PURPOSE;
    let before = composed_frame(&app);
    assert!(before.contains(said), "{before}");
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Down);
    app.update();
    let after = composed_frame(&app);
    assert_eq!(
        after.lines().find(|line| line.contains(said)),
        before.lines().find(|line| line.contains(said)),
        "the line no item stands behind does not take a cursor: {after}"
    );
}

#[test]
fn a_name_shows_in_place_of_the_id_wherever_the_item_is_named() {
    let mut app = fixture_app();
    commit_edit(&mut app, |plan| {
        plan.transfers[0].name = Some("Pension rollover".to_owned());
        plan.events[0].name = Some("Retirement".to_owned());
    });
    show(&mut app, Page::Transfers);
    let frame = composed_frame(&app);
    assert!(frame.contains("Pension rollover"), "the table: {frame}");
    show(&mut app, Page::Events);
    let frame = composed_frame(&app);
    assert!(frame.contains("Retirement"), "the event's own row: {frame}");
    show(&mut app, Page::Expenses);
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("[ Retirement ▾"),
        "a trigger offers the event by name: {frame}"
    );
    assert!(
        frame.contains("after Retirement"),
        "and the details phrase it by name: {frame}"
    );
}

#[test]
fn what_a_blank_field_stands_for_is_read_out_dimmed() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    let world = app.world_mut();
    let rows: Vec<Entity> = world
        .query_filtered::<&Children, With<DetailsTable>>()
        .iter(world)
        .flat_map(|children| children.iter().skip(1).copied())
        .collect();
    let is_dimmed = |row: &Entity| {
        world
            .get::<UiStyle>(*row)
            .is_some_and(|style| style.0.add_modifier.contains(Modifier::DIM))
    };
    let dimmed = rows.iter().filter(|row| is_dimmed(row)).count();
    let draft = world.resource::<Draft>();
    let form = DOMAINS
        .iter()
        .find(|form| form.domain == Some(DomainId::Accounts))
        .expect("the accounts form");
    let item = (form.item)(draft, 0).expect("the first account");
    let unstated = details::rows(form, &item, &draft.plan)
        .iter()
        .filter(|row| row.is_unstated)
        .count();
    assert!(
        unstated > 0,
        "the fixture's cash account leaves fields blank"
    );
    assert_eq!(dimmed, unstated);
}
