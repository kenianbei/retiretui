//! The themes that can be named, and how a name in the config becomes the
//! theme the shell is drawn in.

use std::io;
use std::path::Path;

use bevy_ecs::prelude::Resource;

use super::Theme;
use super::document::{self, Choice, Painted, TERMINAL, Variant};
use crate::settings::Settings;
use crate::store::Store;

/// The directory beside the settings file that the user's themes are in.
const DIRECTORY: &str = "themes";
const EXTENSION: &str = ".toml";

const EMBEDDED: &[(&str, &str)] = &[
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

/// One theme of the set. One that does not read is kept as what was wrong
/// with it, to be said when it is asked for.
#[derive(Debug)]
pub struct Listed {
    pub slug: String,
    pub origin: Origin,
    pub read: Result<Painted, String>,
}

/// Where a theme comes from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origin {
    Embedded,
    User,
    /// The user's, in place of the embedded theme of its name.
    UserOverEmbedded,
}

impl Listed {
    fn of(slug: &str, origin: Origin, text: Result<&str, String>) -> Self {
        let read = text.and_then(document::read);
        Self {
            slug: slug.to_owned(),
            origin,
            read: read.map_err(|error| format!("theme {slug}: {error}")),
        }
    }
}

/// Every theme that can be named, but the terminal's own.
#[derive(Resource, Debug)]
pub struct Themes(Vec<Listed>);

impl Themes {
    pub fn embedded() -> Self {
        let listed = EMBEDDED
            .iter()
            .map(|(slug, text)| Listed::of(slug, Origin::Embedded, Ok(text)));
        Self(listed.collect())
    }

    /// The set a session under `settings` names from, and what is wrong
    /// with the directory its own themes are in.
    pub fn beside(settings: &Settings) -> (Self, Vec<String>) {
        settings.beside(DIRECTORY).map_or_else(
            || (Self::embedded(), Vec::new()),
            |(store, directory)| Self::load(store, &directory),
        )
    }

    /// The embedded set with each theme file of `directory` added by its
    /// stem, one named as an embedded theme is taking its place. A
    /// directory that is not there adds nothing and is not complained of.
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
            match name.strip_suffix(EXTENSION) {
                None | Some("") => {}
                Some(TERMINAL) => complaints.push(format!(
                    "{}: {TERMINAL} is the terminal's own colours; name the file otherwise",
                    path.display()
                )),
                Some(slug) => themes.add(slug, store.read(&path)),
            }
        }
        (themes, complaints)
    }

    fn add(&mut self, slug: &str, text: io::Result<String>) {
        let text = text.as_deref().map_err(ToString::to_string);
        match self.0.iter_mut().find(|listed| listed.slug == slug) {
            Some(embedded) => *embedded = Listed::of(slug, Origin::UserOverEmbedded, text),
            None => self.0.push(Listed::of(slug, Origin::User, text)),
        }
    }

    /// Every name a theme is chosen by, the terminal's own first; that one
    /// is no document.
    pub fn listed(&self) -> impl Iterator<Item = (&str, Option<&Listed>)> {
        let documents = self.0.iter().map(|listed| (&*listed.slug, Some(listed)));
        std::iter::once((TERMINAL, None)).chain(documents)
    }

    /// The theme `choice` names, painted as it asks. A name that is a
    /// family takes the `wanted` variant of it, or whichever it has.
    ///
    /// # Errors
    ///
    /// Where no theme or family has the name, the theme named does not
    /// read, or a colour painted over it does not.
    pub fn resolve(&self, choice: &Choice, wanted: Variant) -> Result<Theme, String> {
        let theme = match choice.name.as_deref() {
            None | Some(TERMINAL) => Theme::terminal(),
            Some(name) => self.named(name, wanted)?,
        };
        choice.over(theme)
    }

    fn named(&self, name: &str, wanted: Variant) -> Result<Theme, String> {
        let read = self.find(name, wanted)?.read.as_ref();
        read.map(|painted| painted.theme.clone())
            .map_err(Clone::clone)
    }

    /// A theme that does not read is no member of a family, so that it
    /// hides nothing but itself.
    fn find(&self, name: &str, wanted: Variant) -> Result<&Listed, String> {
        let mut of_family = None;
        for listed in &self.0 {
            if listed.slug == name {
                return Ok(listed);
            }
            let Ok(painted) = &listed.read else {
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
    use crate::theme::document::LEAST_SERIES;

    fn chosen(name: &str) -> Choice {
        Choice {
            name: Some(name.to_owned()),
            ..Choice::default()
        }
    }

    #[test]
    fn every_embedded_theme_reads_and_names_enough_series_colours() {
        let themes = Themes::embedded();
        assert_eq!(themes.listed().count(), EMBEDDED.len() + 1);
        for (slug, _) in EMBEDDED {
            let theme = themes
                .named(slug, Variant::Dark)
                .unwrap_or_else(|error| panic!("{error}"));
            assert!(theme.bg.is_some(), "{slug} names a background");
            let terminal = Theme::terminal();
            assert!(
                theme.good != terminal.good && theme.caution != terminal.caution,
                "{slug} names its good and caution"
            );
            assert_ne!(theme.series(0), theme.series(LEAST_SERIES - 1), "{slug}");
        }
    }

    #[test]
    fn the_terminals_own_is_listed_first_and_is_no_document() {
        let themes = Themes::embedded();
        let (slug, listed) = themes.listed().next().unwrap();
        assert!(slug == TERMINAL && listed.is_none());
        let worn = themes.resolve(&Choice::default(), Variant::Light);
        assert_eq!(worn, Ok(Theme::terminal()));
        assert_eq!(themes.resolve(&chosen(TERMINAL), Variant::Dark), worn);
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

    const THEMES: &str = "/config/themes";
    const MINE: &str = "family = \"mine\"\nvariant = \"dark\"\naccent = \"#010203\"\n";
    const MINE_ACCENT: Color = Color::Rgb(1, 2, 3);

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
        let documents = themes.listed().filter_map(|(_, listed)| listed);
        documents
            .filter(|listed| listed.origin != Origin::Embedded)
            .map(|listed| (&*listed.slug, listed.origin))
            .collect()
    }

    #[test]
    fn the_users_themes_are_listed_after_the_embedded_ones_by_their_file_names() {
        let light = MINE.replace("dark", "light").replace("#010203", "#040506");
        let files = [("zebra.toml", MINE), ("day.toml", &*light)];
        let (themes, complaints) = loaded(&files);
        assert_eq!(complaints, [""; 0]);
        let slugs: Vec<&str> = themes.listed().map(|(slug, _)| slug).collect();
        assert_eq!(slugs[EMBEDDED.len() + 1..], ["day", "zebra"]);
        let users = [("day", Origin::User), ("zebra", Origin::User)];
        assert_eq!(origins(&themes), users);
        let accent = |name, wanted| themes.named(name, wanted).unwrap().accent;
        assert_eq!(accent("zebra", Variant::Light), MINE_ACCENT);
        assert_eq!(accent("mine", Variant::Dark), MINE_ACCENT, "by family");
        assert_eq!(accent("mine", Variant::Light), Color::Rgb(4, 5, 6));
    }

    #[test]
    fn a_users_theme_named_as_an_embedded_one_takes_its_place() {
        let (themes, complaints) = loaded(&[("nord.toml", MINE)]);
        assert_eq!(complaints, [""; 0]);
        assert_eq!(origins(&themes), [("nord", Origin::UserOverEmbedded)]);
        assert_eq!(themes.listed().count(), EMBEDDED.len() + 1);
        let place = |themes: &Themes| themes.listed().position(|(slug, _)| slug == "nord");
        assert_eq!(place(&themes), place(&Themes::embedded()));
        let worn = themes.named("nord", Variant::Dark).unwrap();
        assert_eq!(worn.accent, MINE_ACCENT);
        let family = themes.named("mine", Variant::Dark).unwrap();
        assert_eq!(family, worn, "and is of the family it states");
    }

    #[test]
    fn a_users_theme_that_does_not_read_is_listed_as_what_is_wrong_with_it() {
        let files = [("bad.toml", "family = 3"), ("good.toml", MINE)];
        let (themes, complaints) = loaded(&files);
        assert_eq!(complaints, [""; 0]);
        let users = [("bad", Origin::User), ("good", Origin::User)];
        assert_eq!(origins(&themes), users);
        let said = themes.named("bad", Variant::Dark).unwrap_err();
        assert!(said.starts_with("theme bad: "), "{said}");
        assert!(themes.named("good", Variant::Dark).is_ok());
        assert!(themes.named("nord", Variant::Dark).is_ok());
    }

    #[test]
    fn a_file_named_for_the_terminals_own_is_refused_and_said() {
        let (themes, complaints) = loaded(&[("terminal.toml", MINE)]);
        assert_eq!(origins(&themes), []);
        assert!(
            matches!(&complaints[..], [said] if said.starts_with("/config/themes/terminal.toml: ")),
            "{complaints:?}"
        );
        let worn = themes.resolve(&chosen(TERMINAL), Variant::Dark);
        assert_eq!(worn, Ok(Theme::terminal()));
    }

    #[test]
    fn what_is_no_theme_file_is_passed_over_and_no_directory_is_no_complaint() {
        let store = KeyStore::new(Memory::default());
        let absent = Themes::load(&store, Path::new(THEMES));
        assert_eq!((origins(&absent.0), absent.1), (vec![], vec![]));

        let nested = Path::new(THEMES).join("more.toml");
        store.create_dir_all(&nested).unwrap();
        store.write(&nested.join("deep.toml"), MINE).unwrap();
        for name in ["notes.md", ".toml", "mine.toml.bak"] {
            store.write(&Path::new(THEMES).join(name), MINE).unwrap();
        }
        let (themes, complaints) = Themes::load(&store, Path::new(THEMES));
        assert_eq!((origins(&themes), complaints), (vec![], vec![]));
    }

    #[test]
    fn a_themes_directory_that_cannot_be_listed_is_said_and_the_embedded_set_kept() {
        let in_the_way =
            std::env::temp_dir().join(format!("retiretui-themes-file-{}", std::process::id()));
        std::fs::write(&in_the_way, "").unwrap();
        let (themes, complaints) = Themes::load(&DiskStore, &in_the_way);
        std::fs::remove_file(&in_the_way).unwrap();
        assert_eq!(themes.listed().count(), EMBEDDED.len() + 1);
        assert!(
            matches!(&complaints[..], [said] if said.contains("retiretui-themes-file")),
            "{complaints:?}"
        );
    }

    #[test]
    fn a_theme_that_does_not_read_hides_nothing_but_itself() {
        let mut themes = Themes::embedded();
        let broken = "family = \"nord\"\nvariant = \"light\"\nbg = 3\n";
        themes
            .0
            .insert(0, Listed::of("aaa", Origin::User, Ok(broken)));
        let said = themes.named("aaa", Variant::Dark).unwrap_err();
        assert_eq!(said, "theme aaa: bg: not a colour");
        let nord = themes.named("nord", Variant::Light).unwrap();
        assert_eq!(
            nord,
            Themes::embedded().named("nord", Variant::Dark).unwrap()
        );
        assert!(themes.named("tokyo-night", Variant::Dark).is_ok());
    }
}
