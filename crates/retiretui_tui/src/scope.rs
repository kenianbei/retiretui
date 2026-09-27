//! Who a key belongs to before the command table: asked of the widget it
//! was typed at and everything that widget sits in.

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Component, Entity, Query};
use bevy_ecs::system::SystemParam;

/// The claim a widget, or everything inside a container, has on the keys
/// typed at it. The widest claim among a widget and its ancestors holds.
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum KeyScope {
    /// Plain keys are the widget's - a letter is typed, tab walks the
    /// form - and chords are still the command table's.
    Plain,
    /// Every key, chords included: what stands over the page.
    All,
}

#[derive(SystemParam)]
pub struct Scoped<'w, 's> {
    parents: Query<'w, 's, &'static ChildOf>,
    scopes: Query<'w, 's, &'static KeyScope>,
}

impl Scoped<'_, '_> {
    /// The claim on a key typed at `target`; `None` leaves it to the
    /// command table.
    pub fn of(&self, target: Entity) -> Option<KeyScope> {
        std::iter::once(target)
            .chain(self.parents.iter_ancestors(target))
            .filter_map(|entity| self.scopes.get(entity).ok().copied())
            .max()
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::World;
    use bevy_ecs::system::SystemState;

    use super::*;

    #[test]
    fn the_widest_claim_among_a_widget_and_what_it_sits_in_holds() {
        let mut world = World::new();
        let page = world.spawn_empty().id();
        let overlay = world.spawn((KeyScope::All, ChildOf(page))).id();
        let field = world.spawn((KeyScope::Plain, ChildOf(overlay))).id();
        let loose_field = world.spawn((KeyScope::Plain, ChildOf(page))).id();
        let mut state = SystemState::<Scoped>::new(&mut world);
        let scoped = state.get(&world).expect("a valid param");
        assert_eq!(scoped.of(field), Some(KeyScope::All));
        assert_eq!(scoped.of(loose_field), Some(KeyScope::Plain));
        assert_eq!(scoped.of(page), None);
    }
}
