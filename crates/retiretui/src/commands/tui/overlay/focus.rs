//! The keyboard the overlays took, and who is owed it back.

use std::any::TypeId;

use bevy_ecs::entity::Entities;
use bevy_ecs::prelude::{Component, Entity, Resource};
use bevy_input_focus::{FocusCause, InputFocus};

/// One overlay's place on the stack: which overlay took the keyboard, and
/// what held it before.
#[derive(Debug)]
struct Taken {
    owner: TypeId,
    held: Option<Entity>,
}

/// What held the keyboard before each overlay that stands took it, in the
/// order they took it. Overlays nest - a question asked over an open item
/// - so what a close gives back is what the one beneath it had.
#[derive(Resource, Default, Debug)]
pub struct Focus {
    taken: Vec<Taken>,
}

impl Focus {
    /// Remembers what holds the keyboard, to be given back when the
    /// `Marker` overlay closes, and answers with how many overlays stand
    /// under it. An overlay that already stands keeps its place.
    pub fn take<Marker: Component>(&mut self, focus: &InputFocus) -> usize {
        if let Some(at) = self.frame_of::<Marker>() {
            return at;
        }
        self.taken.push(Taken {
            owner: TypeId::of::<Marker>(),
            held: focus.get(),
        });
        self.taken.len() - 1
    }

    /// Whether no overlay stands.
    #[must_use]
    pub fn is_clear(&self) -> bool {
        self.taken.is_empty()
    }

    /// What holds the keyboard beneath every overlay: the page's own
    /// holder, which is what `focus` names while none stands.
    pub fn base(&self, focus: &InputFocus) -> Option<Entity> {
        self.taken
            .first()
            .map_or_else(|| focus.get(), |frame| frame.held)
    }

    /// Names `holder` as what holds the keyboard beneath every overlay. It
    /// is given the keyboard now where none stands, and when the last of
    /// them closes where one does - so a page shown from an overlay is not
    /// handed back the keyboard of the page it replaced.
    pub fn rebase(&mut self, holder: Option<Entity>, focus: &mut InputFocus) {
        if let Some(lowest) = self.taken.first_mut() {
            lowest.held = holder;
            return;
        }
        match holder {
            Some(entity) if focus.get() != Some(entity) => {
                focus.set(entity, FocusCause::Navigated);
            }
            Some(_) => {}
            None => focus.clear(),
        }
    }

    fn frame_of<Marker: Component>(&self) -> Option<usize> {
        let owner = TypeId::of::<Marker>();
        self.taken.iter().position(|frame| frame.owner == owner)
    }

    /// Drops the `Marker` overlay's claim without moving the keyboard, for
    /// one the keyboard has already left by other means. It removes its own
    /// frame rather than the top one, so an overlay closing beneath one
    /// that stands leaves that one the keyboard, and owed what this one
    /// was. With none above, what this one was owed is answered.
    pub fn forget<Marker: Component>(&mut self) -> Option<Entity> {
        let at = self.frame_of::<Marker>()?;
        let closed = self.taken.remove(at);
        match self.taken.get_mut(at) {
            Some(above) => {
                above.held = closed.held;
                None
            }
            None => closed.held,
        }
    }

    /// Takes down the `Marker` overlay's frame and gives the keyboard back
    /// to whatever held it before. What a frame held may be gone; the
    /// keyboard is then left where it is rather than given to a node
    /// nothing routes to.
    pub fn restore<Marker: Component>(&mut self, focus: &mut InputFocus, entities: &Entities) {
        if let Some(previous) = self.forget::<Marker>()
            && entities.contains(previous)
        {
            focus.set(previous, FocusCause::Navigated);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::World;

    use super::*;

    #[derive(Component)]
    struct Under;

    #[derive(Component)]
    struct Above;

    fn holding(entity: Entity) -> InputFocus {
        InputFocus::from_entity(entity)
    }

    #[test]
    fn overlays_give_the_keyboard_back_in_the_order_they_took_it() {
        let mut world = World::new();
        let [view, field] = [world.spawn_empty().id(), world.spawn_empty().id()];
        let mut stack = Focus::default();
        let mut focus = holding(view);
        assert_eq!(stack.take::<Under>(&focus), 0);
        focus.set(field, FocusCause::Navigated);
        assert_eq!(stack.take::<Above>(&focus), 1);
        assert_eq!(stack.take::<Above>(&focus), 1, "standing keeps its place");

        stack.restore::<Above>(&mut focus, world.entities());
        assert_eq!(focus.get(), Some(field));
        stack.restore::<Under>(&mut focus, world.entities());
        assert_eq!(focus.get(), Some(view));
        assert_eq!(stack.take::<Under>(&focus), 0, "its frame is gone");
    }

    #[test]
    fn an_overlay_closing_beneath_another_leaves_it_the_keyboard() {
        let mut world = World::new();
        let [view, field, button] = [(); 3].map(|()| world.spawn_empty().id());
        let mut stack = Focus::default();
        let mut focus = holding(view);
        stack.take::<Under>(&focus);
        focus.set(field, FocusCause::Navigated);
        stack.take::<Above>(&focus);
        focus.set(button, FocusCause::Navigated);

        stack.restore::<Under>(&mut focus, world.entities());
        assert_eq!(focus.get(), Some(button), "the one above still holds it");
        stack.restore::<Above>(&mut focus, world.entities());
        assert_eq!(
            focus.get(),
            Some(view),
            "and is owed what the closed one was"
        );
    }

    #[test]
    fn a_holder_that_is_gone_is_not_given_the_keyboard() {
        let mut world = World::new();
        let [view, button] = [world.spawn_empty().id(), world.spawn_empty().id()];
        let mut stack = Focus::default();
        let mut focus = holding(view);
        stack.take::<Under>(&focus);
        focus.set(button, FocusCause::Navigated);
        world.despawn(view);
        stack.restore::<Under>(&mut focus, world.entities());
        assert_eq!(focus.get(), Some(button));
    }
}
