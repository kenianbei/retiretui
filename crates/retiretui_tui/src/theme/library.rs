//! The themes that can be named, and how a name in the config becomes the
//! theme the shell is drawn in.

use std::io;
use std::path::Path;

use bevy_ecs::prelude::Resource;

use super::Theme;
use super::document::{self, Choice, Painted, Variant};
use crate::journal;
use crate::settings::Settings;
use crate::store::Store;

/// The directory beside the settings file that the user's themes are in.
pub const DIRECTORY: &str = "themes";
pub const SUFFIX: &str = ".toml";
/// What is said of a theme that is listed and does not read.
pub const UNREAD: &str = "does not read";
/// The family worn where the settings name no theme, in the screen's
/// variant.
pub const DEFAULT_FAMILY: &str = "github";

pub const EMBEDDED: &[(&str, &str)] = &[
    (
        "catppuccin-latte",
        include_str!("themes/catppuccin-latte.toml"),
    ),
    (
        "catppuccin-mocha",
        include_str!("themes/catppuccin-mocha.toml"),
    ),
    ("dracula", include_str!("themes/dracula.toml")),
    ("github-dark", include_str!("themes/github-dark.toml")),
    ("github-light", include_str!("themes/github-light.toml")),
    ("gruvbox-dark", include_str!("themes/gruvbox-dark.toml")),
    ("gruvbox-light", include_str!("themes/gruvbox-light.toml")),
    ("iceberg-dark", include_str!("themes/iceberg-dark.toml")),
    ("iceberg-light", include_str!("themes/iceberg-light.toml")),
    ("nord", include_str!("themes/nord.toml")),
    ("solarized-dark", include_str!("themes/solarized-dark.toml")),
    (
        "solarized-light",
        include_str!("themes/solarized-light.toml"),
    ),
    ("tokyo-night", include_str!("themes/tokyo-night.toml")),
];

#[derive(Debug)]
pub struct Listed {
    pub slug: String,
    pub origin: Origin,
    /// Nothing where its document does not read.
    pub read: Option<Painted>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origin {
    Embedded,
    User,
    /// The user's, in place of the embedded theme of its name.
    UserOverEmbedded,
}

/// Every theme that can be named.
#[derive(Resource, Debug)]
pub struct Themes(Vec<Listed>);

/// The embedded default family in the `wanted` variant: what is worn
/// where nothing is named, or what is named does not resolve.
///
/// # Panics
///
/// Where the embedded family does not read, which the test suite rules
/// out.
#[must_use]
pub fn default_theme(wanted: Variant) -> Theme {
    Themes::embedded()
        .named(DEFAULT_FAMILY, wanted)
        .expect("the embedded default family reads")
}

impl Themes {
    pub fn embedded() -> Self {
        let listed = EMBEDDED.iter().map(|(slug, text)| Listed {
            slug: (*slug).to_owned(),
            origin: Origin::Embedded,
            read: document::read(text).ok(),
        });
        Self(listed.collect())
    }

    /// The set a session under `settings` names from. What is wrong with
    /// the themes of the user's own is said.
    pub fn beside(settings: &Settings) -> Self {
        let Some((store, directory)) = settings.beside(DIRECTORY) else {
            return Self::embedded();
        };
        let (themes, complaints) = Self::load(store, &directory);
        complaints.iter().for_each(journal::warn);
        themes
    }

    /// The embedded set and each theme file of `directory` by its stem, with
    /// what is wrong with each that does not read. No directory, no complaint.
    pub fn load(store: &dyn Store, directory: &Path) -> (Self, Vec<String>) {
        let mut themes = Self::embedded();
        let entries = match store.list(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return (themes, Vec::new()),
            Err(error) => return (themes, vec![format!("{}: {error}", directory.display())]),
        };
        let files = entries.iter().filter(|entry| !entry.is_dir);
        let mut names: Vec<&str> = files.map(|entry| entry.name.as_str()).collect();
        names.sort_unstable();
        let mut complaints = Vec::new();
        for name in names {
            let path = directory.join(name);
            let complaint = match name.strip_suffix(SUFFIX) {
                None | Some("") => None,
                Some(slug) => themes.add(slug, store.read(&path)),
            };
            complaints.extend(complaint.map(|said| format!("{}: {said}", path.display())));
        }
        (themes, complaints)
    }

    /// Lists the user's theme `slug`, in place of an embedded one of that
    /// name, and answers why its text does not read where it does not.
    fn add(&mut self, slug: &str, text: io::Result<String>) -> Option<String> {
        let text = text.map_err(|error| error.to_string());
        let read = text.and_then(|text| document::read(&text));
        let unread = read.as_ref().err().cloned();
        let mut listed = Listed {
            slug: slug.to_owned(),
            origin: Origin::User,
            read: read.ok(),
        };
        match self.0.iter_mut().find(|embedded| embedded.slug == slug) {
            Some(embedded) => {
                listed.origin = Origin::UserOverEmbedded;
                *embedded = listed;
            }
            None => self.0.push(listed),
        }
        unread
    }

    /// Every theme a name chooses, in the order they are listed.
    pub fn listed(&self) -> impl Iterator<Item = &Listed> {
        self.0.iter()
    }

    /// The theme `choice` names, painted as it asks. A name that is a
    /// family takes the `wanted` variant of it, or whichever it has, and
    /// no name takes the default family's.
    ///
    /// # Errors
    ///
    /// Where no theme or family has the name, the theme named does not
    /// read, or a colour painted over it does not.
    pub fn resolve(&self, choice: &Choice, wanted: Variant) -> Result<Theme, String> {
        let theme = self.named(choice.name.as_deref().unwrap_or(DEFAULT_FAMILY), wanted)?;
        choice.over(theme)
    }

    fn named(&self, name: &str, wanted: Variant) -> Result<Theme, String> {
        let listed = self.find(name, wanted)?;
        let read = listed.read.as_ref().map(|painted| painted.theme.clone());
        read.ok_or_else(|| format!("theme {:?} {UNREAD}", listed.slug))
    }

    /// The theme `name` names, by itself or as its family. One that does
    /// not read is no member of a family, so that it hides nothing but
    /// itself.
    ///
    /// # Errors
    ///
    /// Where no theme or family has the name.
    pub fn find(&self, name: &str, wanted: Variant) -> Result<&Listed, String> {
        let mut of_family = None;
        for listed in &self.0 {
            if listed.slug == name {
                return Ok(listed);
            }
            let Some(painted) = &listed.read else {
                continue;
            };
            if painted.family == name && (of_family.is_none() || painted.variant == wanted) {
                of_family = Some(listed);
            }
        }
        of_family.ok_or_else(|| format!("no theme is named {name:?}"))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use plurimus::core::ratatui_core::style::Color;

    use super::*;
    use crate::store::memory::Memory;
    use crate::store::{DiskStore, KeyStore};
    use crate::support::{USER_THEME, USER_THEME_ACCENT, scratch_dir};
    use crate::theme::document::LEAST_SERIES;

    const THEMES: &str = "/config/themes";
    const BROKEN: &str = "family = \"gruvbox\"\nvariant = \"dark\"\nbg = 3\n";

    fn chosen(name: &str) -> Choice {
        Choice {
            name: Some(name.to_owned()),
            ..Choice::default()
        }
    }

    /// The set over a store holding `files` in its themes directory.
    fn loaded(files: &[(&str, &str)]) -> (Themes, Vec<String>) {
        let store = KeyStore::new(Memory::default());
        store.create_dir_all(Path::new(THEMES)).unwrap();
        for (name, text) in files {
            store.write(&Path::new(THEMES).join(name), text).unwrap();
        }
        Themes::load(&store, Path::new(THEMES))
    }

    fn origins(themes: &Themes) -> Vec<(&str, Origin)> {
        themes
            .listed()
            .filter(|listed| listed.origin != Origin::Embedded)
            .map(|listed| (&*listed.slug, listed.origin))
            .collect()
    }

    #[test]
    fn every_embedded_theme_reads_and_names_enough_series_colours() {
        let themes = Themes::embedded();
        assert_eq!(themes.listed().count(), EMBEDDED.len());
        for (slug, text) in EMBEDDED {
            let read = document::read(text).unwrap_or_else(|error| panic!("{slug}: {error}"));
            let theme = themes.named(slug, Variant::Dark).unwrap();
            assert_eq!(theme, read.theme);
            assert!(theme.bg.is_some(), "{slug} names a background");
            let bare = Theme::bare();
            assert!(
                theme.good != bare.good && theme.caution != bare.caution,
                "{slug} names its good and caution"
            );
            assert_ne!(theme.series(0), theme.series(LEAST_SERIES - 1), "{slug}");
        }
    }

    #[test]
    fn no_choice_wears_the_default_family_in_the_variant_the_screen_wants() {
        let themes = Themes::embedded();
        for (variant, slug) in [
            (Variant::Light, "github-light"),
            (Variant::Dark, "github-dark"),
        ] {
            let worn = themes.resolve(&Choice::default(), variant).unwrap();
            assert_eq!(worn, themes.named(slug, variant).unwrap(), "{slug}");
            assert_eq!(default_theme(variant), worn);
            assert!(
                worn.bg.is_some() && worn.stripe.is_some(),
                "{slug} grounds itself"
            );
        }
    }

    #[test]
    fn a_family_takes_the_variant_the_terminal_wants() {
        let themes = Themes::embedded();
        let named = |name, wanted| themes.named(name, wanted).unwrap();
        let light = named("gruvbox", Variant::Light);
        let dark = named("gruvbox", Variant::Dark);
        assert_eq!(light, named("gruvbox-light", Variant::Dark));
        assert_eq!(dark, named("gruvbox-dark", Variant::Light));
        let only = named("nord", Variant::Light);
        assert_eq!(only, named("nord", Variant::Dark), "or the one it has");
    }

    #[test]
    fn a_choice_paints_over_the_theme_it_names() {
        let choice = Choice {
            transparent: true,
            overrides: BTreeMap::from([("accent".to_owned(), "#fabd2f".to_owned())]),
            ..chosen("nord")
        };
        let theme = Themes::embedded().resolve(&choice, Variant::Dark).unwrap();
        assert_eq!(theme.accent, Color::Rgb(0xfa, 0xbd, 0x2f));
        assert_eq!(theme.bg, None, "transparent drops the background");
        assert!(theme.stripe.is_some(), "and keeps the rows' grounds");
    }

    #[test]
    fn what_does_not_resolve_says_where() {
        let themes = Themes::embedded();
        let unresolved = |choice| themes.resolve(&choice, Variant::Dark).unwrap_err();
        assert!(unresolved(chosen("gruvbocks")).contains("gruvbocks"));
        let miscoloured = Choice {
            overrides: BTreeMap::from([("accent".to_owned(), "mauve-ish".to_owned())]),
            ..Choice::default()
        };
        assert!(unresolved(miscoloured).starts_with("accent:"));
        let misroled = Choice {
            overrides: BTreeMap::from([("surface".to_owned(), "red".to_owned())]),
            ..Choice::default()
        };
        assert!(unresolved(misroled).starts_with("surface:"));
    }

    #[test]
    fn the_users_themes_are_listed_after_the_embedded_ones_by_their_file_names() {
        let light = USER_THEME
            .replace("dark", "light")
            .replace("#010203", "#040506");
        let files = [("zebra.toml", USER_THEME), ("day.toml", &*light)];
        let (themes, complaints) = loaded(&files);
        assert_eq!(complaints, [""; 0]);
        let slugs: Vec<&str> = themes.listed().map(|listed| &*listed.slug).collect();
        assert_eq!(slugs[EMBEDDED.len()..], ["day", "zebra"]);
        let users = [("day", Origin::User), ("zebra", Origin::User)];
        assert_eq!(origins(&themes), users);
        let accent = |name, wanted| themes.named(name, wanted).unwrap().accent;
        assert_eq!(accent("zebra", Variant::Light), USER_THEME_ACCENT);
        assert_eq!(
            accent("mine", Variant::Dark),
            USER_THEME_ACCENT,
            "by family"
        );
        assert_eq!(accent("mine", Variant::Light), Color::Rgb(4, 5, 6));
    }

    #[test]
    fn a_users_theme_named_as_an_embedded_one_takes_its_place() {
        let (themes, complaints) = loaded(&[("nord.toml", USER_THEME)]);
        assert_eq!(complaints, [""; 0]);
        assert_eq!(origins(&themes), [("nord", Origin::UserOverEmbedded)]);
        assert_eq!(themes.listed().count(), EMBEDDED.len());
        let place = |themes: &Themes| themes.listed().position(|listed| listed.slug == "nord");
        assert_eq!(place(&themes), place(&Themes::embedded()));
        let worn = themes.named("nord", Variant::Dark).unwrap();
        assert_eq!(worn.accent, USER_THEME_ACCENT);
        let family = themes.named("mine", Variant::Dark).unwrap();
        assert_eq!(family, worn, "and is of the family it states");
    }

    #[test]
    fn a_users_theme_that_does_not_read_is_listed_and_said_by_its_file() {
        let files = [("bad.toml", "family = 3"), ("good.toml", USER_THEME)];
        let (themes, complaints) = loaded(&files);
        assert!(
            matches!(&complaints[..], [said] if said.starts_with("/config/themes/bad.toml: ")),
            "{complaints:?}"
        );
        let users = [("bad", Origin::User), ("good", Origin::User)];
        assert_eq!(origins(&themes), users);
        let said = themes.named("bad", Variant::Dark).unwrap_err();
        assert_eq!(said, "theme \"bad\" does not read");
        assert!(themes.named("good", Variant::Dark).is_ok());
        assert!(themes.named("nord", Variant::Dark).is_ok());
    }

    #[test]
    fn a_users_theme_that_does_not_read_over_an_embedded_one_is_said_as_it_leaves_its_family() {
        let (themes, complaints) = loaded(&[("gruvbox-dark.toml", BROKEN)]);
        let said = "/config/themes/gruvbox-dark.toml: bg: not a colour";
        assert_eq!(complaints, [said]);
        assert_eq!(
            origins(&themes),
            [("gruvbox-dark", Origin::UserOverEmbedded)]
        );
        assert!(themes.named("gruvbox-dark", Variant::Dark).is_err());
        let family = themes.named("gruvbox", Variant::Dark).unwrap();
        assert_eq!(
            family,
            themes.named("gruvbox-light", Variant::Dark).unwrap()
        );
    }

    #[test]
    fn what_is_no_theme_file_is_passed_over_and_no_directory_is_no_complaint() {
        let store = KeyStore::new(Memory::default());
        let absent = Themes::load(&store, Path::new(THEMES));
        assert_eq!((origins(&absent.0), absent.1), (vec![], vec![]));

        let nested = Path::new(THEMES).join("more.toml");
        store.create_dir_all(&nested).unwrap();
        store.write(&nested.join("deep.toml"), USER_THEME).unwrap();
        for name in ["notes.md", ".toml", "mine.toml.bak"] {
            let file = Path::new(THEMES).join(name);
            store.write(&file, USER_THEME).unwrap();
        }
        let (themes, complaints) = Themes::load(&store, Path::new(THEMES));
        assert_eq!((origins(&themes), complaints), (vec![], vec![]));
    }

    #[test]
    fn a_themes_directory_that_cannot_be_listed_is_said_and_the_embedded_set_kept() {
        let in_the_way = scratch_dir().join(DIRECTORY);
        std::fs::write(&in_the_way, "").unwrap();
        let (themes, complaints) = Themes::load(&DiskStore, &in_the_way);
        assert_eq!(themes.listed().count(), EMBEDDED.len());
        let path = in_the_way.display().to_string();
        assert!(
            matches!(&complaints[..], [said] if said.starts_with(&path)),
            "{complaints:?}"
        );
    }

    #[test]
    fn a_theme_that_does_not_read_hides_nothing_but_itself() {
        let mut themes = Themes::embedded();
        let broken = Listed {
            slug: "aaa".to_owned(),
            origin: Origin::User,
            read: None,
        };
        themes.0.insert(0, broken);
        assert!(themes.named("aaa", Variant::Dark).is_err());
        let nord = themes.named("nord", Variant::Light).unwrap();
        let embedded = Themes::embedded().named("nord", Variant::Dark).unwrap();
        assert_eq!(nord, embedded);
        assert!(themes.named("tokyo-night", Variant::Dark).is_ok());
    }
}
