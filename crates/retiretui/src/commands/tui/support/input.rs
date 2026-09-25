//! Driving a headless app as a user would: keys, the pointer, commands
//! and the clock.

use std::time::Duration;

use bevy_app::App;
use bevy_time::TimeUpdateStrategy;
use plurimus::core::ratatui_core::layout::Position;
use plurimus::term::{
    KeyCode, KeyKind, KeyMessage, KeyModifiers, MouseButton, MouseKind, MouseMessage,
};

use crate::commands::tui::command::Registry;

/// Queues a key press, then ticks the app twice so resource changes made by
/// key handlers reach the layout and render systems.
pub fn press_key(app: &mut App, code: KeyCode) {
    press_with(app, code, KeyModifiers::default());
}

pub fn press_ctrl(app: &mut App, code: KeyCode) {
    press_with(app, code, KeyModifiers::default().with_ctrl(true));
}

/// A shifted press, which is how the terminal layer delivers back-tab.
pub fn press_shift(app: &mut App, code: KeyCode) {
    press_with(app, code, KeyModifiers::default().with_shift(true));
}

fn press_with(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    app.world_mut()
        .write_message(KeyMessage::new(code, modifiers, KeyKind::Press));
    app.update();
    app.update();
}

fn point(app: &mut App, kind: MouseKind, column: u16, row: u16) {
    let position = Position::new(column, row);
    app.world_mut()
        .write_message(MouseMessage::new(kind, position, KeyModifiers::default()));
    app.update();
    app.update();
}

/// Runs the command named `name` through the registry, for where no key
/// reaches it: an open item keeps the plain keys, and the palette with
/// them.
pub fn invoke(app: &mut App, name: &str) {
    let command = crate::commands::tui::command::named(name)
        .unwrap_or_else(|| panic!("no command named {name}"));
    app.world_mut().resource_scope(
        |world, registry: bevy_ecs::change_detection::Mut<Registry>| {
            registry.run(&mut world.commands(), command);
        },
    );
    app.update();
}

/// Runs the command named `name` from the palette.
pub fn run_command(app: &mut App, name: &str) {
    press_key(app, KeyCode::Char(':'));
    type_text(app, name);
    press_key(app, KeyCode::Enter);
    app.update();
}

pub fn hover(app: &mut App, column: u16, row: u16) {
    point(app, MouseKind::Moved, column, row);
}

pub fn click(app: &mut App, column: u16, row: u16) {
    point(app, MouseKind::Down(MouseButton::Left), column, row);
    point(app, MouseKind::Up(MouseButton::Left), column, row);
}

/// Lets `by` pass in one frame, then ticks once more at the clock's own
/// pace so what that frame moved is drawn.
pub fn let_pass(app: &mut App, by: Duration) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(by));
    app.update();
    app.insert_resource(TimeUpdateStrategy::Automatic);
    app.update();
}

/// Types `text` one key press per character.
pub fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        press_key(app, KeyCode::Char(character));
    }
}
