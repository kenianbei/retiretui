//! What the body draws one of at a time: a page of the open document, or
//! nothing, while there is no document.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Query, Res, World};
use bevy_ecs::system::SystemParam;
use bevy_ui::{Display, FlexDirection, Node};

use super::{ActivePage, Page};
use crate::commands::tui::layout::{filling, set_display};
use crate::commands::tui::session::Session;

/// What the body is showing, which is the one question the shell asks of
/// its document: the active page while one is open, and nothing while
/// none is, which is what the new plan's form stands over.
#[derive(SystemParam)]
pub struct ShownSurface<'w> {
    active: Res<'w, ActivePage>,
    session: Res<'w, Session>,
}

/// What is drawn, for a caller holding the world rather than running as
/// a system of its own.
pub fn shown_in(world: &World) -> Option<Page> {
    showing(
        world.resource::<Session>().is_empty(),
        *world.resource::<ActivePage>(),
    )
}

/// The one rule, whichever way its inputs were reached.
const fn showing(is_empty: bool, active: ActivePage) -> Option<Page> {
    if is_empty { None } else { Some(active.0) }
}

impl ShownSurface<'_> {
    #[must_use]
    pub fn surface(&self) -> Option<Page> {
        showing(self.session.is_empty(), *self.active)
    }

    /// Whether anything the surface is read from moved this frame.
    #[must_use]
    pub fn is_changed(&self) -> bool {
        self.active.is_changed() || self.session.is_changed()
    }
}

/// The node a surface lays itself out under. A surface not shown takes no
/// room, so nothing beneath its root is drawn or reached by the pointer.
#[derive(Component, Clone, Copy, Debug)]
pub struct SurfaceRoot(pub Option<Page>);

/// A widget the keyboard walks to on its surface. Where it stands in the
/// walk is where it stands in the drawing: the stops under a surface's
/// root are walked depth first in child order, which is the order the
/// layout draws them in.
#[derive(Component, Clone, Copy, Debug)]
pub struct FocusStop;

/// Spawns `surface`'s root under the body, its contents stacked down it.
pub fn spawn_surface(commands: &mut Commands, body: Entity, surface: Option<Page>) -> Entity {
    commands
        .spawn((
            SurfaceRoot(surface),
            Node {
                flex_direction: FlexDirection::Column,
                display: Display::None,
                ..filling()
            },
            ChildOf(body),
        ))
        .id()
}

/// The surface on show takes the body, and every other root none of it.
pub(super) fn show_the_surface(shown: ShownSurface, mut roots: Query<(&SurfaceRoot, &mut Node)>) {
    if !shown.is_changed() {
        return;
    }
    let shown = shown.surface();
    for (root, mut node) in &mut roots {
        set_display(&mut node, root.0 == shown);
    }
}
