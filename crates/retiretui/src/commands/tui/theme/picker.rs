//! The theme picker: each theme is drawn as the cursor reaches it, kept on
//! enter, and put back on escape.

use bevy_app::{App, Startup};
use bevy_ecs::prelude::{In, Res, ResMut, Resource, World};

use super::Theme;
use super::document::{self, Choice};
use crate::commands::tui::command::Outcome;
use crate::commands::tui::journal;
use crate::commands::tui::picker::{Offered, Picker, Picking, ranked};
use crate::commands::tui::settings::Settings;

pub fn plugin(app: &mut App) {
    app.init_resource::<Worn>();
    app.add_systems(Startup, register);
}

const THEME_KEY: [&str; 2] = ["theme", "name"];

#[derive(Resource, Clone, Copy)]
pub struct ThemePicker(Picker);

/// The theme that was on when the picker opened, to put back.
#[derive(Resource, Default)]
pub struct Worn(Option<Theme>);

fn register(world: &mut World) {
    let picker = Picker::new(world, ("Theme", "theme"), list, keep).trying(world, try_on, restore);
    world.insert_resource(ThemePicker(picker));
}

/// The `theme` command.
pub fn open(
    picker: Res<ThemePicker>,
    theme: Res<Theme>,
    mut worn: ResMut<Worn>,
    mut picking: ResMut<Picking>,
) -> Outcome {
    worn.0 = Some(theme.clone());
    picking.open(picker.0);
    Outcome::Done
}

fn list(In(query): In<String>) -> Vec<Offered> {
    let offered = document::listed().enumerate().map(|(id, (slug, variant))| {
        let badge = variant.map_or("", document::Variant::name);
        Offered::new(id, slug).badged(badge)
    });
    ranked(&query, offered)
}

/// The user's choice with the theme at `id` named in place of theirs, so
/// that what they paint over a theme is tried on with it.
fn choice_of(id: usize, settings: &Settings) -> Option<Choice> {
    let (slug, _) = document::listed().nth(id)?;
    Some(Choice {
        name: Some(slug.to_owned()),
        ..settings.theme.clone()
    })
}

fn try_on(In(id): In<usize>, settings: Res<Settings>, mut theme: ResMut<Theme>) {
    let tried = choice_of(id, &settings)
        .and_then(|choice| document::resolve(&choice, document::wanted_variant()).ok());
    if let Some(tried) = tried
        && *theme != tried
    {
        *theme = tried;
    }
}

fn restore(mut worn: ResMut<Worn>, mut theme: ResMut<Theme>) {
    if let Some(was) = worn.0.take() {
        *theme = was;
    }
}

fn keep(In(id): In<usize>, mut settings: ResMut<Settings>, mut theme: ResMut<Theme>) {
    let Some(choice) = choice_of(id, &settings) else {
        return;
    };
    match document::resolve(&choice, document::wanted_variant()) {
        Ok(kept) => *theme = kept,
        Err(error) => return journal::warn(error),
    }
    let name = choice.name.clone().unwrap_or_default();
    settings.theme = choice;
    match settings.keep(&THEME_KEY, name.as_str()) {
        Ok(()) => journal::say(format!("theme {name}")),
        Err(error) => journal::warn(format!("theme {name}, for this session only: {error}")),
    }
}
