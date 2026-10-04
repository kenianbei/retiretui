//! The themes that can be named, and how a name in the config becomes the
//! theme the shell is drawn in.

use bevy_ecs::prelude::Resource;

use super::Theme;
use super::document::{self, Choice, Painted, TERMINAL, Variant};

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
    pub read: Result<Painted, String>,
}

impl Listed {
    fn of(slug: &str, text: &str) -> Self {
        Self {
            slug: slug.to_owned(),
            read: document::read(text).map_err(|error| format!("theme {slug}: {error}")),
        }
    }
}

/// Every theme that can be named, but the terminal's own.
#[derive(Resource, Debug)]
pub struct Themes(Vec<Listed>);

impl Themes {
    pub fn embedded() -> Self {
        let listed = EMBEDDED.iter().map(|(slug, text)| Listed::of(slug, text));
        Self(listed.collect())
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

    #[test]
    fn a_theme_that_does_not_read_hides_nothing_but_itself() {
        let mut themes = Themes::embedded();
        let broken = Listed::of("aaa", "family = \"nord\"\nvariant = \"light\"\nbg = 3\n");
        themes.0.insert(0, broken);
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
