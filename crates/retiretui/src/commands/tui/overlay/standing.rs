//! The cycle an overlay runs: take down what stands, take the keyboard
//! where nothing did, give it back when it closes.

use bevy_app::Propagate;
use bevy_ecs::entity::Entities;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Query, ResMut, With};
use bevy_ecs::system::SystemParam;
use bevy_input_focus::{FocusCause, InputFocus};

use super::Focus;
use super::band::Band;
use crate::commands::tui::layout::Body;
use crate::commands::tui::motion::Arriving;
use crate::commands::tui::scope::KeyScope;

/// One kind of overlay, spawned as it opens and despawned as it closes.
/// `Marker` is the component its root is marked with. Taking the keyboard
/// and giving it back are the two ends of one stack, and an overlay that
/// reaches them through [`Standing::open`] and [`Standing::close`] has no
/// path that takes without drawing.
#[derive(SystemParam)]
pub struct Standing<'w, 's, Marker: Component + Default> {
    open: Query<'w, 's, Entity, With<Marker>>,
    body: Query<'w, 's, Entity, With<Body>>,
    focus: ResMut<'w, InputFocus>,
    taken: ResMut<'w, Focus>,
    entities: &'w Entities,
}

impl<Marker: Component + Default> Standing<'_, '_, Marker> {
    /// Takes down what stands and answers with the overlay's new root,
    /// marked, hung under the body, and carrying the band everything under
    /// it is painted in. `None` is a shell with no body to hang one under.
    pub fn open(&mut self, commands: &mut Commands) -> Option<Entity> {
        self.take_down(commands);
        let Ok(body) = self.body.single() else {
            self.take_back();
            return None;
        };
        let beneath = self.taken.take::<Marker>(&self.focus);
        let root = commands
            .spawn((
                Marker::default(),
                Arriving,
                KeyScope::All,
                ChildOf(body),
                Propagate(Band::at(beneath)),
            ))
            .id();
        Some(root)
    }

    pub fn is_open(&self) -> bool {
        !self.open.is_empty()
    }

    /// Takes down what stands and gives the keyboard back.
    pub fn close(&mut self, commands: &mut Commands) {
        self.take_down(commands);
        self.take_back();
    }

    /// Gives the keyboard to what the overlay drew.
    pub fn focus(&mut self, inner: Entity) {
        self.focus.set(inner, FocusCause::Navigated);
    }

    fn take_down(&mut self, commands: &mut Commands) {
        for open in &self.open {
            commands.entity(open).despawn();
        }
    }

    fn take_back(&mut self) {
        self.taken.restore::<Marker>(&mut self.focus, self.entities);
    }
}
