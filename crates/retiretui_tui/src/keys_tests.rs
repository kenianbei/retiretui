//! Headless tests of the keys the shell shares with its widgets: a copy
//! chord beside quit, and an arrow at the end of a list.

use crate::support::typed;
use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::In;
use bevy_input::keyboard::Key;
use bevy_input_focus::InputFocus;
use plurimus::term::LastCopied;

use super::nav::Page;
use super::support::{SIZE, headless_app, press_ctrl, press_key, show};

fn copied(app: &App) -> String {
    let last = app.world().resource::<LastCopied>();
    last.0.clone().unwrap_or_default()
}

fn holder(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

#[test]
fn ctrl_c_copies_nothing_from_a_grid_and_ctrl_q_quits() {
    let mut app = headless_app(SIZE);
    for page in [Page::Accounts, Page::Ledger] {
        show(&mut app, page);
        press_ctrl(&mut app, typed('c'));
        assert_eq!(copied(&app), "", "{page:?}'s grid copies nothing");
        assert!(app.should_exit().is_none(), "a copy is not a quit");
    }
    press_ctrl(&mut app, typed('q'));
    assert!(app.should_exit().is_some());
}

fn assert_ends_keep_the_keyboard(app: &mut App, list: &str) {
    let pane = holder(app);
    for (end, past) in [(Key::Home, Key::ArrowUp), (Key::End, Key::ArrowDown)] {
        press_key(app, end);
        press_key(app, past.clone());
        assert_eq!(holder(app), pane, "{list} {past:?}");
    }
}

#[test]
fn an_arrow_at_the_end_of_a_list_keeps_the_keyboard() {
    let mut app = headless_app(SIZE);
    for page in [Page::Accounts, Page::Ledger, Page::TaxTables] {
        show(&mut app, page);
        assert_ends_keep_the_keyboard(&mut app, &format!("{page:?}"));
    }
    press_key(&mut app, Key::Escape);
    assert_ends_keep_the_keyboard(&mut app, "the sidebar");
}

#[test]
fn ctrl_c_on_a_list_copies_the_row_under_its_cursor() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    let mut sidebars = app
        .world_mut()
        .query_filtered::<(Entity, &plurimus::widgets::ActiveDescendant), bevy_ecs::prelude::With<crate::sidebar::Sidebar>>();
    let list = sidebars
        .iter(app.world())
        .find_map(|(list, cursor)| cursor.0.map(|_| list))
        .expect("the Plan tab's sidebar is on its page");
    app.world_mut()
        .insert_resource(InputFocus::from_entity(list));
    app.update();
    press_ctrl(&mut app, typed('c'));
    assert!(copied(&app).starts_with("Accounts"), "{:?}", copied(&app));
}

/// On a terminal reporting no key releases, a modifier is let up by a
/// timeout on the real clock; a chord after it still carries Ctrl.
#[test]
fn a_second_ctrl_chord_after_the_release_timeout_keeps_ctrl() {
    use crate::support::commit_edit;
    use plurimus::term::{KeyModifiers, TerminalInput};

    fn chord(In(state): In<bevy_input::ButtonState>, mut input: TerminalInput) {
        let ctrl = KeyModifiers::default().with_ctrl(true);
        input.key(typed('z'), state, false, ctrl);
    }
    let mut app = headless_app(SIZE);
    commit_edit(&mut app, |plan| plan.plan.name = Some("first".to_owned()));
    commit_edit(&mut app, |plan| plan.plan.name = Some("second".to_owned()));
    for _ in 0..2 {
        for state in [
            bevy_input::ButtonState::Pressed,
            bevy_input::ButtonState::Released,
        ] {
            app.world_mut()
                .run_system_cached_with(chord, state)
                .unwrap();
            app.update();
        }
        std::thread::sleep(std::time::Duration::from_millis(700));
    }
    let name = app
        .world()
        .resource::<crate::edit::Draft>()
        .plan
        .plan
        .name
        .clone();
    assert_eq!(name.as_deref(), Some("test-plan"), "both undos landed");
}
