//! The pickers the command table opens: commands by name, commands by what
//! they do, and pages.

use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::prelude::{In, Res, ResMut, Resource, World};

use super::{CommandId, Outcome, Pending};
use crate::commands::tui::nav::{ActivePage, Page};
use crate::commands::tui::picker::{Offered, Picker, Picking, ranked};

/// The pickers, registered once the world can hold their systems.
#[derive(Resource, Clone, Copy)]
pub struct Pickers {
    commands: Picker,
    help: Picker,
    pages: Picker,
}

pub fn register(world: &mut World) {
    let pickers = Pickers {
        commands: Picker::new(world, ("Commands", "command"), list_names, run_chosen),
        help: Picker::new(world, ("Help", "command"), list_docs, run_chosen),
        pages: Picker::new(world, ("Go to", "page"), list_pages, show_chosen),
    };
    world.insert_resource(pickers);
}

pub fn open_commands(pickers: Res<Pickers>, mut picking: ResMut<Picking>) -> Outcome {
    picking.open(pickers.commands);
    Outcome::Done
}

pub fn open_help(pickers: Res<Pickers>, mut picking: ResMut<Picking>) -> Outcome {
    picking.open(pickers.help);
    Outcome::Done
}

pub fn open_pages(pickers: Res<Pickers>, mut picking: ResMut<Picking>) -> Outcome {
    picking.open(pickers.pages);
    Outcome::Done
}

/// Commands by the name they are run by, each saying what it does.
fn list_names(In(query): In<String>) -> Vec<Offered> {
    let offered = super::all().map(|command| {
        let spec = command.spec();
        Offered::new(command.0, spec.name).badged(spec.doc)
    });
    ranked(&query, offered)
}

/// Commands by what they do, each naming its key.
fn list_docs(In(query): In<String>) -> Vec<Offered> {
    let offered = super::all()
        .map(|command| Offered::new(command.0, command.spec().doc).badged(command.key_label()));
    ranked(&query, offered)
}

fn run_chosen(In(id): In<usize>, mut pending: ResMut<Pending>) {
    pending.defer(CommandId(id));
}

fn list_pages(In(query): In<String>) -> Vec<Offered> {
    let offered = Page::ALL.into_iter().map(|page| {
        Offered::new(page.index(), page.title()).badged(page.heading().unwrap_or_default())
    });
    ranked(&query, offered)
}

fn show_chosen(In(index): In<usize>, mut active: ResMut<ActivePage>) {
    if let Some(&page) = Page::ALL.get(index) {
        active.set_if_neq(ActivePage(page));
    }
}
