//! The notice that stands in for the frame below the size the shell needs.

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Commands, Component, Entity, Query, Res, With};
use bevy_ui::Node;
use plurimus::core::{TerminalSize, UiArea, UiHidden, UiWidget};
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;

use super::layout::{self, Root};
use super::{edit, tabbar};

/// The smallest terminal anything here is laid out for. What the shell
/// derives it needs only ever raises the minimum above this, never below.
const FLOOR: TerminalSize = TerminalSize::new(128, 32);

/// The smallest terminal the shell lays itself out in: what the tab row
/// and the least form, one field over its foot, need, or the floor where
/// they need less; a taller form scrolls.
/// Derived so that a renamed tab or a domain that gains a field cannot
/// leave it silently stale.
const MIN_SIZE: TerminalSize = TerminalSize::new(
    raised(FLOOR.cols, tabbar::TABS_COLS + tabbar::status::MIN_COLS),
    raised(FLOOR.rows, layout::CHROME_ROWS + edit::SHORTEST_FORM_ROWS),
);

const fn raised(floor: u16, needed: u16) -> u16 {
    if needed > floor { needed } else { floor }
}

/// The line shown in place of the frame below [`MIN_SIZE`].
#[derive(Component, Debug)]
struct Notice;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_notice);
    app.add_systems(Update, guard_size);
}

fn spawn_notice(mut commands: Commands) {
    let notice = format!(
        "terminal too small (minimum {}x{})",
        MIN_SIZE.cols, MIN_SIZE.rows
    );
    commands.spawn((
        Notice,
        UiWidget::new(Paragraph::new(notice)),
        UiArea::Fill,
        UiHidden,
    ));
}

fn fits(size: TerminalSize) -> bool {
    size.cols >= MIN_SIZE.cols && size.rows >= MIN_SIZE.rows
}

fn guard_size(
    size: Res<TerminalSize>,
    mut roots: Query<&mut Node, With<Root>>,
    notices: Query<Entity, With<Notice>>,
    mut commands: Commands,
) {
    if !size.is_changed() {
        return;
    }
    let is_fitting = fits(*size);
    for mut root in &mut roots {
        layout::set_display(&mut root, is_fitting);
    }
    for notice in &notices {
        if is_fitting {
            commands.entity(notice).insert(UiHidden);
        } else {
            commands.entity(notice).remove::<UiHidden>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_needs_the_minimum_size() {
        assert!(!fits(TerminalSize::new(127, 32)));
        assert!(!fits(TerminalSize::new(128, 31)));
        assert!(fits(MIN_SIZE));
    }
}
