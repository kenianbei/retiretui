//! The theme picker: each theme is drawn as the cursor reaches it, kept on
//! enter, and put back on escape.

use bevy_app::{App, Startup};
use bevy_ecs::prelude::{In, Res, ResMut, Resource, World};
use bevy_ecs::system::SystemParam;

use super::document::Choice;
use super::library::{Listed, Origin, Themes, UNREAD};
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
    /// The user's choice naming the theme at `id`, so that what they paint
    /// over a theme is tried on with it, and the theme that resolves to.
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

/// The `theme` command. The themes are read again as it opens and the one
/// worn put on afresh, so that one being written shows without a restart.
pub fn open(
    picker: Res<ThemePicker>,
    mut theme: ResMut<Theme>,
    mut shelf: Shelf,
    mut worn: ResMut<Worn>,
    mut picking: ResMut<Picking>,
) -> Outcome {
    *shelf.themes = Themes::beside(&shelf.settings);
    if let Ok(reread) = shelf.themes.resolve(&shelf.settings.theme, shelf.wanted.0)
        && *theme != reread
    {
        *theme = reread;
    }
    worn.0 = Some(theme.clone());
    picking.open(picker.0);
    Outcome::Done
}

fn list(In(query): In<String>, themes: Res<Themes>) -> Vec<Offered> {
    let offered = themes.listed().enumerate().map(|(id, (slug, listed))| {
        let offered = Offered::new(id, slug).badged(listed.map_or_else(String::new, badge));
        match listed {
            Some(Listed { read: None, .. }) => offered.dimmed(),
            _ => offered,
        }
    });
    ranked(&query, offered)
}

fn badge(listed: &Listed) -> String {
    let Some(painted) = &listed.read else {
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

/// A theme that does not resolve is not kept: a press and its release in
/// one frame choose a row the cursor never came to rest on.
fn keep(In(id): In<usize>, mut shelf: Shelf, worn: ResMut<Worn>, mut theme: ResMut<Theme>) {
    let Some((choice, tried)) = shelf.tried(id) else {
        return;
    };
    match tried {
        Ok(kept) => *theme = kept,
        Err(error) => {
            restore(worn, theme);
            return journal::warn(error);
        }
    }
    let name = choice.name.clone().unwrap_or_default();
    shelf.settings.theme = choice;
    match shelf.settings.keep(&THEME_NAME, name.as_str()) {
        Ok(()) => journal::say(format!("theme {name}")),
        Err(error) => journal::warn(format!("theme {name}, for this session only: {error}")),
    }
}
