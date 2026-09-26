//! A grouped tab's sidebar: the pages the tab holds - the plan's domains
//! with how many items each holds, or the tools - kept beside whichever
//! of them is on show. Its cursor and the page are one fact:
//! moving either moves the other.

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Changed, Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, ResMut, With, World,
};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::Key;
use bevy_ui::Node;
use plurimus::core::ratatui_core::layout::Size;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::ScrollArea;
use plurimus::widgets::{
    ActiveDescendant, ListBoxAction, ListBoxKeys, ListItemTrailing, ValueChange, list_item, listbox,
};

use super::command::Outcome;
use super::edit::{self, Draft};
use super::focus::{self, PageFocus, SidebarLed};
use super::hints::Hints;
use super::layout::{self, Body, filling, placed};
use super::nav::{self, Group, LastShown, Page, PageSystems, ShownSurface, Turn};
use super::pane::Pane;
use super::theme::{Repainted, Theme};

/// The cells a sidebar takes of the body: its border, the cursor, and the
/// widest row.
pub const SIDEBAR_COLS: u16 = BORDER_COLS + layout::CURSOR_COLS + widest_row();

const BORDER_COLS: u16 = 2;
/// A count of two digits, with the cell a list keeps clear either side of
/// one.
const COUNT_COLS: usize = 4;

/// The widest grouped page's row: its name, and on a domain's the count.
const fn widest_row() -> u16 {
    let mut widest = 0;
    let mut at = 0;
    while at < Page::ALL.len() {
        let page = Page::ALL[at];
        let count = if page.is_domain() { COUNT_COLS } else { 0 };
        let width = page.title().len() + count;
        if page.group().is_some() && width > widest {
            widest = width;
        }
        at += 1;
    }
    widest as u16
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_sidebars.after(layout::spawn_frame));
    app.add_systems(Update, turn_to_the_cursor.in_set(PageSystems::Turn));
    app.add_systems(
        Update,
        (
            show_the_sidebar,
            point_at_the_page
                .after(focus::settle_focus)
                .run_if(is_pointing_due),
        )
            .in_set(PageSystems::Show),
    );
    app.add_systems(Update, refresh_counts.in_set(Repainted));
    app.add_observer(handle_page_entered);
}

/// The pane a group's sidebar is drawn in, which takes no room on a page
/// that is not one of the group's.
#[derive(Component, Debug)]
struct SidebarPane(Group);

/// A group's list of pages.
#[derive(Component, Clone, Copy, Debug)]
pub struct Sidebar(Group);

/// The page a row of a sidebar shows.
#[derive(Component, Clone, Copy, Debug)]
pub struct SidebarRow(Page);

/// A sidebar's keys: a list's own, and the arrow that goes into the page
/// as choosing its row does.
fn sidebar_keys() -> ListBoxKeys {
    let mut keys = ListBoxKeys::default();
    keys.0.push((Key::ArrowRight.into(), ListBoxAction::Select));
    keys
}

const fn hints(group: Group) -> Hints {
    match group {
        Group::Tools => Hints(&[("↑↓", "tool"), ("→", "into")]),
        Group::Plan => Hints(&[("↑↓", "domain"), ("→", "into")]),
    }
}

fn spawn_sidebars(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
        return;
    };
    for group in Group::ALL {
        let pane = Pane::new(group.title())
            .wide(f32::from(SIDEBAR_COLS))
            .spawn(&mut commands, body);
        commands.entity(pane).insert(SidebarPane(group));
        // Ahead of every page's root, which are the body's other children.
        commands.entity(body).insert_children(0, &[pane]);
        let list = commands
            .spawn((
                Sidebar(group),
                layout::Rests,
                hints(group),
                listbox(),
                sidebar_keys(),
                layout::list_cursor(),
                ScrollArea::new(Size::default()),
                filling(),
                placed(),
                ChildOf(pane),
            ))
            .id();
        for page in Page::ALL
            .into_iter()
            .filter(|page| page.group() == Some(group))
        {
            commands.spawn((list_item(page.title()), SidebarRow(page), ChildOf(list)));
        }
    }
}

/// A sidebar's cursor, read as the page it is on.
#[derive(SystemParam)]
pub struct SidebarCursor<'w, 's> {
    lists: Query<'w, 's, (Entity, &'static Sidebar, &'static mut ActiveDescendant)>,
    rows: Query<'w, 's, (Entity, &'static SidebarRow)>,
}

impl SidebarCursor<'_, '_> {
    /// The group's list and its cursor.
    fn of(&self, group: Group) -> Option<(Entity, &ActiveDescendant)> {
        self.lists
            .iter()
            .find(|(_, sidebar, _)| sidebar.0 == group)
            .map(|(list, _, cursor)| (list, cursor))
    }

    pub fn list(&self, group: Group) -> Option<Entity> {
        self.of(group).map(|(list, _)| list)
    }

    /// Puts the cursor of `page`'s group on its row, leaving one already
    /// there alone.
    pub fn point_at(&mut self, page: Page) {
        let Some(group) = page.group() else {
            return;
        };
        let row = self.rows.iter().find(|(_, row)| row.0 == page);
        let Some((row, _)) = row else {
            return;
        };
        let list = self
            .lists
            .iter_mut()
            .find(|(_, sidebar, _)| sidebar.0 == group);
        if let Some((_, _, mut cursor)) = list
            && cursor.0 != Some(row)
        {
            cursor.0 = Some(row);
        }
    }
}

/// A cursor moved, by key or by pointer: the page follows it, and the
/// sidebar keeps the keyboard through the turn.
fn turn_to_the_cursor(
    moved: Query<(&Sidebar, &ActiveDescendant), Changed<ActiveDescendant>>,
    rows: Query<&SidebarRow>,
    mut turn: Turn,
    mut led: ResMut<SidebarLed>,
) {
    for (sidebar, cursor) in &moved {
        // A sidebar that is not shown had its cursor moved by no one.
        if turn.page().group() != Some(sidebar.0) {
            continue;
        }
        if let Some(row) = cursor.0.and_then(|row| rows.get(row).ok()) {
            turn.to(row.0);
            led.0 = true;
        }
    }
}

/// The page turned some other way, or a turn was refused: the cursor
/// follows it. After the keyboard is settled, which reads a cursor that
/// already names the page as the sidebar having turned it.
fn point_at_the_page(shown: ShownSurface, mut cursor: SidebarCursor) {
    if let Some(page) = shown.surface() {
        cursor.point_at(page);
    }
}

/// The page or a sidebar's cursor moved, which is all a cursor is pointed
/// back from.
fn is_pointing_due(
    shown: ShownSurface,
    moved: Query<(), (With<Sidebar>, Changed<ActiveDescendant>)>,
) -> bool {
    shown.is_changed() || !moved.is_empty()
}

fn show_the_sidebar(shown: ShownSurface, mut panes: Query<(&SidebarPane, &mut Node)>) {
    if !shown.is_changed() {
        return;
    }
    let group = shown.surface().and_then(Page::group);
    for (pane, mut node) in &mut panes {
        layout::set_display(&mut node, group == Some(pane.0));
    }
}

fn refresh_counts(
    draft: Res<Draft>,
    theme: Res<Theme>,
    rows: Query<(Entity, &SidebarRow, Option<&ListItemTrailing>)>,
    mut commands: Commands,
) {
    if !draft.is_changed() && !theme.is_changed() {
        return;
    }
    for (entity, row, drawn) in &rows {
        let Some(count) = edit::item_count(row.0, &draft.plan) else {
            continue;
        };
        let trailing = Line::styled(count.to_string(), theme.dimmed());
        if drawn.is_none_or(|drawn| drawn.0 != trailing) {
            commands.entity(entity).insert(ListItemTrailing(trailing));
        }
    }
}

/// Choosing a row goes into its page, which the cursor already shows.
fn handle_page_entered(
    chosen: On<ValueChange<Entity>>,
    lists: Query<(), With<Sidebar>>,
    mut focus: PageFocus,
) {
    if lists.contains(chosen.source) {
        focus.enter_page();
    }
}

/// A group's tab command: the tab shows the page last shown through it,
/// with the keyboard on the sidebar that chooses between them.
pub fn enter(world: &mut World, group: Group) -> Outcome {
    let last = *world.resource::<LastShown>();
    nav::turn_in(world, nav::entering(group.tab(), last));
    world.resource_mut::<SidebarLed>().0 = true;
    let _ = world.run_system_cached(focus::enter_sidebar);
    Outcome::Done
}

#[cfg(test)]
mod tests;
