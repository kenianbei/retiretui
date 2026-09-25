//! An editing page: the domain's table in a pane where it has many items,
//! and its form alone where it has one.

use bevy_app::{App, PostUpdate, Update};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, With};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_ui::{FlexDirection, Node, UiRect, UiSystems, Val};
use plurimus::widgets::{Activate, button};

use super::details;
use super::domain::{ListOps, Ops};
use super::draft::Draft;
use super::editing::{self, SessionFocus, Slot};
use super::select;
use super::sort;
use super::table::{
    DomainTable, cue_receipt, handle_row_select, handle_table_key, rebuild_rows, table_bundle,
};
use super::trigger;
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::{self, CURSOR_COLS, button_node, filling, placed};
use crate::commands::tui::nav::{self, FocusStop};
use crate::commands::tui::pane::Pane;

pub fn plugin(app: &mut App) {
    app.add_plugins(select::plugin);
    app.add_systems(
        Update,
        (
            (rebuild_rows, details::refresh).chain(),
            trigger::place_slots,
        )
            .in_set(super::EditSystems::Place),
    );
    app.add_systems(Update, dress_add_buttons);
    app.add_systems(PostUpdate, cue_receipt.after(UiSystems::PostLayout));
    app.add_observer(handle_row_select);
    app.add_observer(sort::handle_header_click);
}

/// Answers with the page's root, for a caller that hangs more under it.
pub fn spawn_screen(commands: &mut Commands, body: Entity, ops: Ops) -> Entity {
    let root = nav::spawn_surface(commands, body, ops.surface);
    match ops.list {
        Some(list) => spawn_table(commands, root, ops, list),
        None => spawn_single_details(commands, root, ops),
    }
    root
}

const SINGLE_HINTS: Hints = Hints(&[("⏎", "edit"), ("esc", "domains")]);

/// The cell kept clear between the table's pane and the details'.
const PANE_GAP: f32 = 1.0;

/// The table in a pane, in a row its details join it in where the
/// domain's items say more than the columns show.
fn spawn_table(commands: &mut Commands, root: Entity, ops: Ops, list: ListOps) {
    let beside = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(PANE_GAP),
                ..filling()
            },
            ChildOf(root),
        ))
        .id();
    let pane = Pane::new(ops.title).sharing(1.0).spawn(commands, beside);
    let table = commands
        .spawn((
            table_bundle(),
            DomainTable {
                ops,
                list,
                sort: None,
                wanted: None,
                is_applied: false,
            },
            FocusStop,
            layout::Rests,
            Hints(&[("⏎", "edit"), ("←", "domains")]),
            filling(),
            placed(),
            ChildOf(pane),
        ))
        .observe(handle_table_key)
        .id();
    spawn_add_button(commands, pane, table, list);
    if details::has_details(ops, list) {
        details::spawn_pane(commands, beside, table, ops);
    }
}

/// The affordance that adds an item, under the table it adds to. It is no
/// tab stop: the key row says the key, and a stop here would stand between
/// the table and its details.
fn spawn_add_button(commands: &mut Commands, pane: Entity, table: Entity, list: ListOps) {
    let label = format!("+ Add {}", list.singular);
    let node = Node {
        margin: UiRect::left(Val::Px(f32::from(CURSOR_COLS))),
        ..button_node(&label)
    };
    commands
        .spawn((
            button(label),
            AddButton(table),
            node,
            placed(),
            ChildOf(pane),
        ))
        .insert(TabIndex(layout::NO_STOP))
        .observe(handle_add_press);
}

/// The table the button adds an item to.
#[derive(Component, Clone, Copy)]
struct AddButton(Entity);

/// Opens a new item, as the add command does. `Activate` lands on the
/// button and does not bubble, so it is observed here.
fn handle_add_press(
    activate: On<Activate>,
    buttons: Query<&AddButton>,
    mut state: SessionFocus,
    mut commands: Commands,
) {
    let Ok(button) = buttons.get(activate.entity) else {
        return;
    };
    let Ok(domain) = state.tables.get(button.0) else {
        return;
    };
    let opened = (domain.ops, Some(button.0), Slot::New(domain.list));
    // The table takes the keyboard first: the button is no tab stop, and
    // what a form opens over is what it hands the keyboard back to.
    state.focus(button.0);
    commands.run_system_cached_with(editing::open_item, opened);
}

/// Takes the button out of the page where the document cannot be written
/// to, so nothing offers what the session refuses. Read-only is not fixed
/// for a session: saving a scenario under a new name clears it.
fn dress_add_buttons(draft: Res<Draft>, mut buttons: Query<&mut Node, With<AddButton>>) {
    for mut node in &mut buttons {
        layout::set_display(&mut node, !draft.is_read_only());
    }
}

/// A single-item domain's page: its details, which ⏎ opens the form over.
fn spawn_single_details(commands: &mut Commands, root: Entity, ops: Ops) {
    let pane = Pane::new(ops.title).spawn(commands, root);
    details::spawn_into(commands, pane, ops, None, SINGLE_HINTS);
}
