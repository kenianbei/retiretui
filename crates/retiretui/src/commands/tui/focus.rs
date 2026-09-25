//! Who holds the keyboard among the panes of the page on show.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Added, Entity, IntoScheduleConfigs, Query, ResMut, Resource};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::Key;
use bevy_input_focus::InputFocus;
use bevy_input_focus::directional_navigation::DirectionalNavigationMap;
use bevy_math::CompassOctant;

use super::command::{self, Outcome};
use super::nav::{FocusStop, Page, PageSystems, ShownSurface, SurfaceRoot};
use super::overlay;
use super::sidebar::SidebarCursor;

pub fn plugin(app: &mut App) {
    app.init_resource::<SidebarLed>();
    app.add_systems(
        Update,
        (settle_focus.in_set(PageSystems::Show), block_bound_arrows),
    );
}

/// Whether the sidebar is turning the page this frame, and so keeps the
/// keyboard through the turn: a page's own command turned to from the
/// sidebar goes into the page instead. Set by whatever turns a page from
/// the sidebar, read and cleared when the keyboard settles.
#[derive(Resource, Default, Debug)]
pub struct SidebarLed(pub bool);

/// The panes the keyboard walks on the surface on show.
#[derive(SystemParam)]
pub struct Ring<'w, 's> {
    pub(super) shown: ShownSurface<'w>,
    roots: Query<'w, 's, (Entity, &'static SurfaceRoot)>,
    children: Query<'w, 's, &'static Children>,
    stops: Query<'w, 's, (), bevy_ecs::prelude::With<FocusStop>>,
    pub(super) sidebar: SidebarCursor<'w, 's>,
}

impl Ring<'_, '_> {
    /// The sidebar beside a grouped page.
    fn sidebar_list(&self) -> Option<Entity> {
        self.shown
            .surface()
            .and_then(Page::group)
            .and_then(|group| self.sidebar.list(group))
    }

    /// The page's own panes in the order they are drawn: every stop under
    /// the surface's root, depth first in child order.
    fn panes(&self) -> impl Iterator<Item = Entity> {
        let shown = self.shown.surface();
        let root = self
            .roots
            .iter()
            .find(|(_, root)| root.0 == shown)
            .map(|(entity, _)| entity);
        root.into_iter()
            .flat_map(|root| self.children.iter_descendants_depth_first(root))
            .filter(|&entity| self.stops.contains(entity))
    }

    /// The panes the keyboard walks: the sidebar beside a grouped page,
    /// then the page's own.
    pub fn stops(&self) -> Vec<Entity> {
        self.sidebar_list()
            .into_iter()
            .chain(self.panes())
            .collect()
    }

    /// Whether there is more than one pane to walk.
    pub fn has_many(&self) -> bool {
        let panes = self.panes().take(2).count();
        usize::from(self.sidebar_list().is_some()) + panes > 1
    }
}

/// Who may hold the keyboard on the page on show, and who does.
#[derive(SystemParam)]
pub struct PageFocus<'w, 's> {
    pub(super) ring: Ring<'w, 's>,
    led: ResMut<'w, SidebarLed>,
    focus: ResMut<'w, InputFocus>,
    overlays: ResMut<'w, overlay::Focus>,
}

impl PageFocus<'_, '_> {
    /// Hands the keyboard to the page on show: its first pane.
    pub fn enter_page(&mut self) {
        let first = self.ring.panes().next();
        self.set(first);
    }

    /// What holds the keyboard beneath any overlay that stands.
    pub(super) fn holder(&self) -> Option<Entity> {
        self.overlays.base(&self.focus)
    }

    /// The page's holder is the overlay stack's base, so a page shown from
    /// an overlay gets the keyboard when the overlay closes.
    pub(super) fn set(&mut self, holder: Option<Entity>) {
        let Self {
            overlays, focus, ..
        } = self;
        overlays.rebase(holder, focus);
    }

    /// Hands the keyboard to the pane after the one holding it, or the
    /// one before, wrapping. A page with one pane or none keeps the key.
    fn step(&mut self, step: isize) {
        let stops = self.ring.stops();
        let holder = self.holder();
        let Some(held) = stops.iter().position(|&stop| Some(stop) == holder) else {
            return;
        };
        let next = (held as isize + step).rem_euclid(stops.len() as isize) as usize;
        if next != held {
            self.set(Some(stops[next]));
        }
    }

    /// A page the sidebar turned to leaves the keyboard on the sidebar.
    /// Otherwise the page's first pane takes it, and a page with none
    /// leaves the keyboard nowhere rather than on the chrome.
    fn settle(&mut self, is_led: bool) {
        let sidebar = is_led.then(|| self.ring.sidebar_list()).flatten();
        match sidebar {
            Some(list) => self.set(Some(list)),
            None => self.enter_page(),
        }
    }
}

/// The flag is read every frame, so one raised by a turn that turned
/// nothing does not outlive the frame; reading it marks nothing changed.
pub fn settle_focus(mut focus: PageFocus) {
    let is_led = std::mem::take(&mut focus.led.bypass_change_detection().0);
    if focus.ring.shown.is_changed() {
        focus.settle(is_led);
    }
}

/// The `domains` command: hands the keyboard to the sidebar, its cursor
/// on the page on show. A page of no group keeps the key.
pub fn enter_sidebar(mut focus: PageFocus) -> Outcome {
    if let Some(page) = focus.ring.shown.surface()
        && let Some(group) = page.group()
    {
        focus.ring.sidebar.point_at(page);
        let list = focus.ring.sidebar.list(group);
        focus.set(list);
    }
    Outcome::Done
}

/// Where an arrow key would walk to from a pane.
fn octant(key: &Key) -> Option<CompassOctant> {
    match key {
        Key::ArrowUp => Some(CompassOctant::North),
        Key::ArrowDown => Some(CompassOctant::South),
        Key::ArrowLeft => Some(CompassOctant::West),
        Key::ArrowRight => Some(CompassOctant::East),
        _ => None,
    }
}

/// An arrow a command particular to the page binds does not also walk
/// the keyboard off a pane of that page: the arrow navigation hears the
/// key beside the command table, and a key the table ran cannot be
/// stopped from reaching it.
fn block_bound_arrows(
    stops: Query<Entity, Added<FocusStop>>,
    parents: Query<&ChildOf>,
    roots: Query<&SurfaceRoot>,
    mut map: ResMut<DirectionalNavigationMap>,
) {
    for stop in &stops {
        let page = parents
            .iter_ancestors(stop)
            .find_map(|ancestor| roots.get(ancestor).ok())
            .and_then(|root| root.0);
        let bound = page.into_iter().flat_map(command::arrows_bound_on);
        for direction in bound.filter_map(octant) {
            map.block_edge(stop, direction);
        }
    }
}

/// The `focus-next` command: on to the next pane.
pub fn focus_next(mut focus: PageFocus) -> Outcome {
    focus.step(1);
    Outcome::Done
}

/// The `focus-previous` command: back to the pane before.
pub fn focus_previous(mut focus: PageFocus) -> Outcome {
    focus.step(-1);
    Outcome::Done
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::SystemState;
    use plurimus::bui::ComputedNodeRect;

    use super::*;
    use crate::commands::tui::hints::Hints;
    use crate::commands::tui::nav::Page;
    use crate::commands::tui::support::{SIZE, headless_app, show};

    fn ring(app: &mut bevy_app::App) -> Vec<Entity> {
        let mut state = SystemState::<Ring>::new(app.world_mut());
        let ring = state.get_mut(app.world_mut()).expect("a valid param");
        ring.stops()
    }

    /// The first word each stop hints for `↑↓` or `⏎`, which names it.
    fn named(app: &bevy_app::App, stops: &[Entity]) -> Vec<&'static str> {
        stops
            .iter()
            .map(|&stop| {
                app.world()
                    .get::<Hints>(stop)
                    .and_then(|hints| hints.0.first())
                    .map_or("", |(_, word)| word)
            })
            .collect()
    }

    #[test]
    fn every_page_walks_its_panes_in_the_order_they_are_drawn() {
        let mut app = headless_app(SIZE);
        for page in Page::ALL {
            show(&mut app, page);
            let stops = ring(&mut app);
            let panes = &stops[usize::from(page.group().is_some())..];
            let placed: Vec<(u16, u16, Entity)> = panes
                .iter()
                .map(|&stop| {
                    let rect = app.world().get::<ComputedNodeRect>(stop).expect("laid out");
                    (rect.visible.y, rect.visible.x, stop)
                })
                .collect();
            let mut by_place = placed.clone();
            by_place.sort_unstable();
            assert_eq!(placed, by_place, "{page:?} is walked out of drawn order");
        }
    }

    #[test]
    fn the_tool_pages_are_walked_from_the_left() {
        let mut app = headless_app(SIZE);
        show(&mut app, Page::RothConversions);
        let stops = ring(&mut app);
        assert_eq!(named(&app, &stops), ["tool", "edit", "option", "year"]);
        show(&mut app, Page::MonteCarlo);
        let stops = ring(&mut app);
        assert_eq!(named(&app, &stops), ["tool", "assumption", "run"]);
    }
}
