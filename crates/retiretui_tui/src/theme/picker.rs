//! The theme picker: each theme is drawn as the cursor reaches it, kept on
//! enter, and put back on escape.

use bevy_app::{App, Startup};
use bevy_ecs::prelude::{In, Res, ResMut, Resource, World};
use bevy_ecs::system::SystemParam;

use super::document::Choice;
use super::library::{Listed, Origin, Themes};
use super::{Theme, WantedVariant};
use crate::command::Outcome;
use crate::journal;
use crate::picker::{Offered, Picker, Picking, ranked};
use crate::settings::{self, Settings};

pub fn plugin(app: &mut App) {
    app.init_resource::<Worn>();
    app.add_systems(Startup, register);
}

const THEME_NAME: [&str; 2] = [settings::THEME_KEY, "name"];
const UNREAD: &str = "does not read";
const YOURS: &str = "yours";

#[derive(Resource, Clone, Copy)]
pub struct ThemePicker(Picker);

/// The theme that was on when the picker opened, to put back.
#[derive(Resource, Default)]
pub struct Worn(Option<Theme>);

fn register(world: &mut World) {
    let picker = Picker::new(world, ("Theme", "theme"), list, keep).trying(world, try_on, restore);
    world.insert_resource(ThemePicker(picker));
}

/// The themes on offer, and the settings a choice among them is made
/// under.
#[derive(SystemParam)]
pub struct Shelf<'w> {
    settings: ResMut<'w, Settings>,
    themes: ResMut<'w, Themes>,
    wanted: Res<'w, WantedVariant>,
}

impl Shelf<'_> {
    /// The user's choice with the theme at `id` named in place of theirs,
    /// so that what they paint over a theme is tried on with it, and the
    /// theme that choice resolves to.
    fn tried(&self, id: usize) -> Option<(Choice, Result<Theme, String>)> {
        let (slug, _) = self.themes.listed().nth(id)?;
        let choice = Choice {
            name: Some(slug.to_owned()),
            ..self.settings.theme.clone()
        };
        let theme = self.themes.resolve(&choice, self.wanted.0);
        Some((choice, theme))
    }
}

/// The `theme` command. The user's themes are read again as it opens, so
/// that one being written is tried on without a restart.
pub fn open(
    picker: Res<ThemePicker>,
    theme: Res<Theme>,
    mut shelf: Shelf,
    mut worn: ResMut<Worn>,
    mut picking: ResMut<Picking>,
) -> Outcome {
    let (themes, complaints) = Themes::beside(&shelf.settings);
    *shelf.themes = themes;
    complaints.iter().for_each(journal::warn);
    worn.0 = Some(theme.clone());
    picking.open(picker.0);
    Outcome::Done
}

fn list(In(query): In<String>, themes: Res<Themes>) -> Vec<Offered> {
    let offered = themes.listed().enumerate().map(|(id, (slug, listed))| {
        Offered::new(id, slug).badged(listed.map_or_else(String::new, badge))
    });
    ranked(&query, offered)
}

fn badge(listed: &Listed) -> String {
    let Ok(painted) = &listed.read else {
        return UNREAD.to_owned();
    };
    let variant = painted.variant.name();
    match listed.origin {
        Origin::Embedded => variant.to_owned(),
        Origin::User | Origin::UserOverEmbedded => format!("{variant}, {YOURS}"),
    }
}

/// A theme that does not resolve shows what was on when the picker opened.
fn try_on(In(id): In<usize>, shelf: Shelf, worn: Res<Worn>, mut theme: ResMut<Theme>) {
    let resolved = shelf.tried(id).and_then(|(_, tried)| tried.ok());
    if let Some(tried) = resolved.or_else(|| worn.0.clone())
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

fn keep(In(id): In<usize>, mut shelf: Shelf, mut theme: ResMut<Theme>) {
    let Some((choice, tried)) = shelf.tried(id) else {
        return;
    };
    match tried {
        Ok(kept) => *theme = kept,
        Err(error) => return journal::warn(error),
    }
    let name = choice.name.clone().unwrap_or_default();
    shelf.settings.theme = choice;
    match shelf.settings.keep(&THEME_NAME, name.as_str()) {
        Ok(()) => journal::say(format!("theme {name}")),
        Err(error) => journal::warn(format!("theme {name}, for this session only: {error}")),
    }
}
