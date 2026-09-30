//! The Tax Tables page's pickers: a filing status or a state in place of
//! the plan's own, each tried on as the cursor reaches it.

use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::prelude::{In, Res, ResMut, Resource, World};
use retiretui_client::forms::offers::Offer;
use retiretui_client::tax_tables::{STATE_PICK, STATUS_PICK, YearTables};

use super::{Tabled, TaxPicks};
use crate::command::Outcome;
use crate::picker::{Offered, Picker, Picking, ranked};

/// Which of the view a picker picks.
#[derive(Clone, Copy, Debug)]
pub enum Pick {
    Status,
    State,
}

impl Pick {
    fn slot(self, picks: &mut TaxPicks) -> &mut Option<String> {
        match self {
            Self::Status => &mut picks.status,
            Self::State => &mut picks.state,
        }
    }

    /// What the picker offers after the plan's own row.
    fn offers(self, tables: &YearTables) -> (&str, &[Offer]) {
        match self {
            Self::Status => (&tables.own_status, &tables.statuses),
            Self::State => (&tables.own_state, &tables.states),
        }
    }

    /// The value the row `id` stands for: none for the plan's own row.
    fn value_at(self, tables: &YearTables, id: usize) -> Option<String> {
        let (_, offers) = self.offers(tables);
        let at = id.checked_sub(1)?;
        offers.get(at).map(|offer| offer.value.clone())
    }
}

/// The page's two pickers.
#[derive(Resource, Clone, Copy)]
pub struct TaxPickers {
    status: Picker,
    state: Picker,
}

/// The picks when a picker opened, put back where it closes with nothing
/// chosen.
#[derive(Resource, Default)]
pub struct Held(TaxPicks);

pub(super) fn register(world: &mut World) {
    let status = picker(world, Pick::Status, (STATUS_PICK, "filing status"));
    let state = picker(world, Pick::State, (STATE_PICK, "state"));
    world.insert_resource(TaxPickers { status, state });
}

fn picker(world: &mut World, pick: Pick, named: (&'static str, &'static str)) -> Picker {
    let list = move |In(query): In<String>, tabled: Res<Tabled>| {
        let Some(tables) = &tabled.0 else {
            return Vec::new();
        };
        let (own, offers) = pick.offers(tables);
        let labels = std::iter::once(own).chain(offers.iter().map(|offer| offer.label.as_str()));
        ranked(
            &query,
            labels
                .enumerate()
                .map(|(id, label)| Offered::new(id, label)),
        )
    };
    let set = move |In(id): In<usize>, tabled: Res<Tabled>, picks: ResMut<TaxPicks>| {
        if let Some(tables) = &tabled.0 {
            let value = pick.value_at(tables, id);
            picks
                .map_unchanged(|picks| pick.slot(picks))
                .set_if_neq(value);
        }
    };
    let restore = |mut held: ResMut<Held>, mut picks: ResMut<TaxPicks>| {
        picks.set_if_neq(std::mem::take(&mut held.0));
    };
    Picker::new(world, named, list, set).trying(world, set, restore)
}

/// The command that opens `pick`'s picker.
pub fn opens(
    pick: Pick,
) -> impl FnMut(Res<TaxPickers>, Res<TaxPicks>, ResMut<Held>, ResMut<Picking>) -> Outcome {
    move |pickers, picks, mut held, mut picking| {
        held.0.clone_from(&picks);
        picking.open(match pick {
            Pick::Status => pickers.status,
            Pick::State => pickers.state,
        });
        Outcome::Done
    }
}
