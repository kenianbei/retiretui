//! The message drawer: everything said this session, drawn up from the
//! bottom of the body.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, ResMut, Resource, With,
};
use bevy_input::keyboard::KeyboardInput;
use bevy_input_focus::FocusedInput;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{ModalDismiss, first_bound};
use plurimus::widgets::{ActiveDescendant, list_item};
use tracing::Level;

use super::command::Outcome;
use super::hints::Hints;
use super::journal::Journal;
use super::overlay::{self, Standing};
use super::picker;
use super::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.init_resource::<Drawer>();
    app.add_systems(Update, sync_drawer.in_set(Repainted).after(picker::Synced));
}

const TITLE: &str = "Messages";
const NOTHING_SAID: &str = "nothing has been said yet";
const HINTS: Hints = Hints(&[("↑↓", "scroll"), ("esc", "close")]);

/// Whether the drawer is open.
#[derive(Resource, Default)]
pub struct Drawer(bool);

#[derive(Component, Default, Debug)]
struct DrawerRoot;

#[derive(Component)]
struct EntryList;

/// The `messages` command: opens the drawer, or closes it.
pub fn toggle(mut drawer: ResMut<Drawer>) -> Outcome {
    drawer.0 = !drawer.0;
    Outcome::Done
}

fn sync_drawer(
    state: (Res<Drawer>, Res<Journal>, Res<Theme>),
    lists: Query<Entity, With<EntryList>>,
    mut standing: Standing<DrawerRoot>,
    mut commands: Commands,
) {
    let (drawer, journal, theme) = state;
    if !drawer.is_changed() {
        // Something said while the drawer is open joins the list; the
        // drawer itself stands as it stood.
        if let (true, Ok(list)) = (drawer.0 && journal.is_changed(), lists.single()) {
            commands.entity(list).despawn_related::<Children>();
            let newest = spawn_entries(&mut commands, list, &journal, &theme);
            commands.entity(list).insert(ActiveDescendant(newest));
        }
        return;
    }
    if !drawer.0 {
        standing.close(&mut commands);
        return;
    }
    let Some(root) = standing.open(&mut commands) else {
        return;
    };
    let list = overlay::bottom_panel(&mut commands, root, TITLE, HINTS);
    commands
        .entity(root)
        .observe(handle_key)
        .observe(handle_dismiss);
    commands.entity(list).insert(EntryList);
    let newest = spawn_entries(&mut commands, list, &journal, &theme);
    commands.entity(list).insert(ActiveDescendant(newest));
    standing.focus(list);
}

/// One row per entry, oldest first, answering with the newest: where the
/// cursor opens, so the list is scrolled to what was just said.
fn spawn_entries(
    commands: &mut Commands,
    list: Entity,
    journal: &Journal,
    theme: &Theme,
) -> Option<Entity> {
    let mut newest = None;
    for entry in journal.entries() {
        let said = if entry.level <= Level::WARN {
            theme.exceeded()
        } else {
            theme.ui_theme().normal
        };
        let line = Line::from(vec![
            Span::styled(entry.at.strftime("%H:%M  ").to_string(), theme.dimmed()),
            Span::styled(entry.text.clone(), said),
        ]);
        newest = Some(commands.spawn((list_item(line), ChildOf(list))).id());
    }
    if newest.is_none() {
        let line = Line::styled(NOTHING_SAID, theme.dimmed());
        commands.spawn((list_item(line), ChildOf(list)));
    }
    newest
}

fn handle_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    mut drawer: ResMut<Drawer>,
) {
    if first_bound(overlay::CLOSE_KEYS, &input.input, held.get()).is_some() {
        input.propagate(false);
        drawer.0 = false;
    }
}

fn handle_dismiss(_dismissed: On<ModalDismiss>, mut drawer: ResMut<Drawer>) {
    drawer.0 = false;
}
