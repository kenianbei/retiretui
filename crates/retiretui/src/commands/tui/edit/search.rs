//! A pick from a set too long for a menu: the select's button opens the
//! shell's fuzzy picker over its offers, and takes the choice as a menu's.

use bevy_ecs::prelude::{Commands, Entity, In, On, Query, Res, ResMut, Resource, World};
use plurimus::widgets::Activate;

use super::offers::Vocabulary;
use super::select::{PickChanged, Select};
use crate::commands::tui::picker::{Offered, Picker, Picking, ranked};

/// The pickers a searched select opens, and the select the one on show
/// answers to.
#[derive(Resource)]
pub struct Searches {
    countries: Picker,
    states: Picker,
    target: Option<Entity>,
}

pub fn register(world: &mut World) {
    let searches = Searches {
        countries: Picker::new(world, ("Country", "country"), list_offers, take_chosen),
        states: Picker::new(world, ("State", "state"), list_offers, take_chosen),
        target: None,
    };
    world.insert_resource(searches);
}

pub fn handle_search(
    activate: On<Activate>,
    selects: Query<&Select>,
    mut searches: ResMut<Searches>,
    mut picking: ResMut<Picking>,
) {
    let Some(vocabulary) = selects.get(activate.entity).ok().and_then(Select::searched) else {
        return;
    };
    let picker = match vocabulary {
        Vocabulary::Country => searches.countries,
        Vocabulary::UsState => searches.states,
        _ => return,
    };
    searches.target = Some(activate.entity);
    picking.open(picker);
}

/// The offers by the words shown for them, each beside the value kept; the
/// row past the last empties a field that may be empty.
fn list_offers(
    In(query): In<String>,
    searches: Res<Searches>,
    selects: Query<&Select>,
) -> Vec<Offered> {
    let Some(select) = searches.target.and_then(|target| selects.get(target).ok()) else {
        return Vec::new();
    };
    let (offers, blank) = select.offered();
    let listed = offers.iter().enumerate();
    let rows = listed.map(|(at, offer)| Offered::new(at, &offer.label).badged(&offer.value));
    let emptying = blank.map(|blank| Offered::new(offers.len(), blank).dimmed());
    ranked(&query, rows.chain(emptying))
}

fn take_chosen(
    In(chosen): In<usize>,
    searches: Res<Searches>,
    mut selects: Query<&mut Select>,
    mut commands: Commands,
) {
    let Some(entity) = searches.target else {
        return;
    };
    let Ok(mut select) = selects.get_mut(entity) else {
        return;
    };
    let value = select.choose(Some(chosen));
    commands.trigger(PickChanged { entity, value });
}
