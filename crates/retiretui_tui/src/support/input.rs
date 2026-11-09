//! Driving a headless app as a user would: keys, the pointer, commands
//! and the clock.

use std::time::Duration;

use bevy_app::App;
use bevy_ecs::prelude::{Entity, In, With};
use bevy_input::ButtonState;
use bevy_input::keyboard::Key;
use bevy_math::Vec2;
use bevy_time::TimeUpdateStrategy;
use bevy_window::{PrimaryWindow, Window, WindowRef};
use plurimus::core::bevy_camera::NormalizedRenderTarget;
use plurimus::core::ratatui_core::layout::Position;
use plurimus::term::{KeyModifiers, MouseButton, TerminalInput, VirtualWindow};
use plurimus::ui::bevy_picking::backend::HitData;
use plurimus::ui::bevy_picking::events::{Pointer, PointerClick};
use plurimus::ui::bevy_picking::pointer::{Location, PointerButton, PointerId};

use crate::command::Registry;

/// The key `character` is typed as.
#[must_use]
pub fn typed(character: char) -> Key {
    Key::Character(character.encode_utf8(&mut [0; 4]).into())
}

/// What a terminal reports, in the sink's terms.
enum Report {
    Key(Key, ButtonState),
    Button(ButtonState, Position),
    Moved(Position),
    WheelDown(Position),
}

fn inject(In((report, modifiers)): In<(Report, KeyModifiers)>, mut input: TerminalInput) {
    match report {
        Report::Key(key, state) => input.key(key, state, false, modifiers),
        Report::Button(state, at) => input.button(MouseButton::Left, state, at, modifiers),
        Report::Moved(at) => input.moved(at, modifiers),
        Report::WheelDown(at) => input.wheel(at, 0, -1, modifiers),
    }
}

/// Queues `report` as a backend writes it, through one cached sink so the
/// cell it last placed the pointer in carries across calls. The window it
/// writes for is spawned on the first frame, so one written before any is
/// spawned here.
fn report(app: &mut App, report: Report, modifiers: KeyModifiers) {
    let world = app.world_mut();
    let has_window = world
        .query_filtered::<(), With<PrimaryWindow>>()
        .single(world)
        .is_ok();
    if !has_window {
        world.spawn(VirtualWindow);
    }
    world
        .run_system_cached_with(inject, (report, modifiers))
        .expect("the sink runs against the terminal's resources");
}

/// Presses and releases a key, ticking after each so resource changes made
/// by key handlers reach the layout and render systems.
pub fn press_key(app: &mut App, key: Key) {
    press_with(app, key, KeyModifiers::default());
}

pub fn press_ctrl(app: &mut App, key: Key) {
    press_with(app, key, KeyModifiers::default().with_ctrl(true));
}

pub fn press_alt(app: &mut App, key: Key) {
    press_with(app, key, KeyModifiers::default().with_alt(true));
}

/// A shifted press, which is how the terminal layer delivers back-tab.
pub fn press_shift(app: &mut App, key: Key) {
    press_with(app, key, KeyModifiers::default().with_shift(true));
}

fn press_with(app: &mut App, key: Key, modifiers: KeyModifiers) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        report(app, Report::Key(key.clone(), state), modifiers);
        app.update();
    }
}

fn point(app: &mut App, pointing: Report) {
    report(app, pointing, KeyModifiers::default());
    app.update();
    app.update();
}

/// Runs the command named `name` through the registry, for where no key
/// reaches it: an open item keeps the plain keys, and the palette with
/// them.
pub fn invoke(app: &mut App, name: &str) {
    let command = crate::command::named(name).unwrap_or_else(|| panic!("no command named {name}"));
    app.world_mut().resource_scope(
        |world, registry: bevy_ecs::change_detection::Mut<Registry>| {
            registry.run(&mut world.commands(), command);
        },
    );
    app.update();
}

/// Runs the command named `name` from the palette.
pub fn run_command(app: &mut App, name: &str) {
    press_key(app, typed(':'));
    type_text(app, name);
    press_key(app, Key::Enter);
    app.update();
}

pub fn hover(app: &mut App, column: u16, row: u16) {
    point(app, Report::Moved(Position::new(column, row)));
}

/// One wheel tick down at a cell.
pub fn scroll_down(app: &mut App, column: u16, row: u16) {
    point(app, Report::WheelDown(Position::new(column, row)));
}

/// A click of the window's pointer on `entity`, at `position` in pixels.
/// The event climbs to the root and then to the pointer's window, which
/// has to be one, or it climbs from there again.
pub fn click_entity(app: &mut App, entity: Entity, position: Vec2) {
    let window = app.world_mut().spawn(Window::default()).id();
    let target = WindowRef::Primary.normalize(Some(window)).unwrap();
    let location = Location {
        target: NormalizedRenderTarget::Window(target),
        position,
    };
    app.world_mut().trigger(PointerClick {
        entity,
        pointer: Pointer::new(PointerId::Mouse, location),
        button: PointerButton::Primary,
        hit: HitData::new(entity, 0.0, None, None),
        duration: Duration::ZERO,
        count: 1,
    });
}

pub fn click(app: &mut App, column: u16, row: u16) {
    let at = Position::new(column, row);
    point(app, Report::Button(ButtonState::Pressed, at));
    point(app, Report::Button(ButtonState::Released, at));
}

/// A press and its release in one frame, as a tap sends them.
pub fn tap(app: &mut App, column: u16, row: u16) {
    let at = Position::new(column, row);
    report(
        app,
        Report::Button(ButtonState::Pressed, at),
        KeyModifiers::default(),
    );
    point(app, Report::Button(ButtonState::Released, at));
}

/// Lets `by` pass in one frame, then ticks once more at the clock's own
/// pace so what that frame moved is drawn.
pub fn let_pass(app: &mut App, by: Duration) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(by));
    app.update();
    app.insert_resource(TimeUpdateStrategy::Automatic);
    app.update();
}

/// Writes `text` over `path` a moment after its last write, so its mtime
/// moves, and lets the watch's beat come round.
pub fn change_on_disk(app: &mut App, path: &std::path::Path, text: &str) {
    std::thread::sleep(Duration::from_millis(50));
    std::fs::write(path, text).unwrap();
    let_pass(app, Duration::from_secs_f32(crate::watch::POLL_SECONDS));
}

/// Types `text` one key press per character.
pub fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        press_key(app, typed(character));
    }
}
