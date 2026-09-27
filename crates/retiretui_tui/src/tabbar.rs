//! The tab row: the shell's six destinations, boxed along the top of the
//! frame, and the status kept beside them.

pub mod status;

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, Has, IntoScheduleConfigs, On, Query, Res, With,
};
use bevy_ecs::system::EntityCommands;
use bevy_ui::{Node, Val};
use plurimus::core::Edge;
use plurimus::core::ratatui_core::style::Modifier;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::ui::{Checked, InteractionDisabled, PressFocusDisabled, UiLabel, ValueChange};
use plurimus::widgets::ratatui_widgets::borders::BorderType;
use plurimus::widgets::{TabBarActiveStyle, TabBarLook, tab_bar, tab_item};

use super::layout::{self, TabRow, placed};
use super::nav::{self, LastShown, Page, ShownSurface, TAB_COUNT, Turn};
use super::session::Session;
use super::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_tab_row.after(layout::spawn_frame));
    app.add_systems(Update, (light_the_active_tab, disable_the_dead_tabs));
    app.add_systems(Update, repaint_tabs.in_set(Repainted));
    app.add_observer(handle_tab_chosen);
    app.add_plugins(status::plugin);
}

/// Cells a boxed tab takes beside its label: its border, each side. The
/// bar is drawn unpadded because six padded tabs and the status do not
/// share the smallest terminal the shell lays out in.
const TAB_DECORATION: u16 = 2;

/// Cells the label spends on the digit that selects the tab.
const DIGIT_COLS: u16 = 2;

/// Columns the tabs take together, which is what the row reserves for
/// them before the status gets any.
pub const TABS_COLS: u16 = tabs_cols();

const _: () = assert!(TAB_COUNT < 10, "a tab is labelled by one digit");

const fn tabs_cols() -> u16 {
    let mut cols = 0;
    let mut tab = 0;
    while tab < TAB_COUNT {
        cols += TAB_DECORATION + DIGIT_COLS + nav::tab_title(tab).len() as u16;
        tab += 1;
    }
    cols
}

/// The bar itself, which the shell has one of.
#[derive(Component, Debug)]
struct ShellTabs;

/// Which tab an item of the bar stands for. The bar's items are
/// positional: [`nav::ActivePage`] is what a tab means, both ways.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct BarTab(pub(super) usize);

/// Each tab boxed, the active box opening onto a baseline drawn along the
/// row's foot.
fn look() -> TabBarLook {
    TabBarLook::default()
        .with_border(Some(BorderType::Rounded))
        .with_joined(Some(Edge::Bottom))
        .with_padding(0)
}

fn spawn_tab_row(rows: Query<Entity, With<TabRow>>, theme: Res<Theme>, mut commands: Commands) {
    let Ok(row) = rows.single() else {
        return;
    };
    let bar = commands
        .spawn((
            ShellTabs,
            Node {
                flex_grow: 1.0,
                height: Val::Percent(100.0),
                ..Node::default()
            },
            tab_bar(),
            look(),
            TabBarActiveStyle(active_style(&theme)),
            PressFocusDisabled,
            placed(),
            ChildOf(row),
        ))
        .id();
    for tab in 0..TAB_COUNT {
        commands.spawn((tab_item(tab_label(tab, &theme)), BarTab(tab), ChildOf(bar)));
    }
    status::spawn(&mut commands, row);
}

fn active_style(theme: &Theme) -> plurimus::core::ratatui_core::style::Style {
    theme.accented().add_modifier(Modifier::BOLD)
}

/// The digit that selects the tab, dimmed ahead of its title.
fn tab_label(tab: usize, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{} ", nav::tab_digit(tab)), theme.dimmed()),
        Span::raw(nav::tab_title(tab)),
    ])
}

fn repaint_tabs(
    theme: Res<Theme>,
    mut bars: Query<&mut TabBarActiveStyle, With<ShellTabs>>,
    mut items: Query<(&BarTab, &mut UiLabel)>,
) {
    if !theme.is_changed() {
        return;
    }
    for mut style in &mut bars {
        style.0 = active_style(&theme);
    }
    for (tab, mut label) in &mut items {
        label.0 = tab_label(tab.0, &theme);
    }
}

/// The bar shows which tab the surface on show is reached through: the
/// `Plan` tab while there is no document, where the new plan's form
/// stands.
fn light_the_active_tab(
    shown: ShownSurface,
    items: Query<(Entity, &BarTab, Has<Checked>)>,
    mut commands: Commands,
) {
    if !shown.is_changed() {
        return;
    }
    let lit = shown.surface().map_or(nav::Group::Plan.tab(), Page::tab);
    for (entity, tab, is_marked) in &items {
        mark(commands.entity(entity), Checked, tab.0 == lit, is_marked);
    }
}

/// Puts `marker` on the item, or takes it off, leaving alone what is
/// already as it should be so the bar redraws no more than it must.
fn mark<Marker: Component>(mut item: EntityCommands, marker: Marker, should_be: bool, is: bool) {
    if should_be == is {
        return;
    }
    if should_be {
        item.insert(marker);
    } else {
        item.remove::<Marker>();
    }
}

/// Without a document only the `Plan` tab has anything to show; the bar
/// skips a disabled item in both its input and its styling.
fn disable_the_dead_tabs(
    session: Res<Session>,
    items: Query<(Entity, &BarTab, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if !session.is_changed() {
        return;
    }
    let is_empty = session.is_empty();
    for (entity, tab, is_marked) in &items {
        let is_dead = is_empty && tab.0 != nav::Group::Plan.tab();
        mark(
            commands.entity(entity),
            InteractionDisabled,
            is_dead,
            is_marked,
        );
    }
}

fn handle_tab_chosen(
    chosen: On<ValueChange<Entity>>,
    bars: Query<(), With<ShellTabs>>,
    items: Query<&BarTab>,
    last: Res<LastShown>,
    mut turn: Turn,
) {
    if !bars.contains(chosen.source) {
        return;
    }
    let Ok(tab) = items.get(chosen.value) else {
        return;
    };
    turn.to(nav::entering(tab.0, *last));
}

#[cfg(test)]
mod tests;
