//! Themes as documents: the embedded set, the colour syntax, and how a
//! name in the config becomes the theme the shell is drawn in.

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::LazyLock;

use plurimus::core::ratatui_core::style::Color;
use serde::Deserialize;

use super::Theme;

/// The name of the theme that is the terminal's own colours.
pub const TERMINAL: &str = "terminal";

/// What a colour is spelled as where it means the terminal's own.
const DEFAULT_COLOUR: &str = "default";
const LEAST_SERIES: usize = 4;
const GROUND_VARIABLE: &str = "COLORFGBG";
const GROUND_SEPARATOR: char = ';';
/// The backgrounds a terminal names that are dark; the rest are light.
const DARK_GROUNDS: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 8];

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

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    Dark,
    Light,
}

impl Variant {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

/// A theme as it is written: every role a colour by name. A key that is
/// not a role is refused as the roles are painted.
#[derive(Deserialize, Debug)]
struct Document {
    family: String,
    variant: Variant,
    #[serde(flatten)]
    roles: BTreeMap<String, toml::Value>,
}

/// The theme a config asks for: which one, and how it is painted over.
#[derive(Deserialize, Default, Clone, PartialEq, Eq, Debug)]
pub struct Choice {
    /// A theme, or a family of them; the terminal's own where unnamed.
    #[serde(default)]
    pub name: Option<String>,
    /// Drop the theme's background for the terminal's own.
    #[serde(default)]
    pub transparent: bool,
    /// Any role, painted over the named theme.
    #[serde(flatten)]
    pub overrides: BTreeMap<String, String>,
}

/// The embedded documents, read once. One that does not read is kept as
/// what was wrong with it, to be said when it is asked for.
static DOCUMENTS: LazyLock<Vec<(&'static str, Result<Document, String>)>> = LazyLock::new(|| {
    EMBEDDED
        .iter()
        .map(|(slug, text)| {
            let read = toml::from_str(text).map_err(|error| format!("theme {slug}: {error}"));
            (*slug, read)
        })
        .collect()
});

/// Every theme that can be named, with the variant it is of; the
/// terminal's own first.
pub fn listed() -> impl Iterator<Item = (&'static str, Option<Variant>)> {
    let embedded = DOCUMENTS.iter().filter_map(|(slug, document)| {
        let document = document.as_ref().ok()?;
        Some((*slug, Some(document.variant)))
    });
    std::iter::once((TERMINAL, None)).chain(embedded)
}

/// The variant `COLORFGBG` says the terminal wants, and dark where it says
/// nothing.
pub fn wanted_variant() -> Variant {
    let ground = std::env::var(GROUND_VARIABLE).ok();
    variant_of(ground.as_deref())
}

fn variant_of(ground: Option<&str>) -> Variant {
    let named = ground
        .and_then(|ground| ground.rsplit(GROUND_SEPARATOR).next())
        .and_then(|ground| ground.parse::<u8>().ok());
    match named {
        Some(ground) if !DARK_GROUNDS.contains(&ground) => Variant::Light,
        _ => Variant::Dark,
    }
}

/// The theme `choice` names, painted as it asks. A name that is a family
/// takes the `wanted` variant of it, or whichever it has.
///
/// # Errors
///
/// Where no theme or family has the name, or a colour does not read.
pub fn resolve(choice: &Choice, wanted: Variant) -> Result<Theme, String> {
    let name = choice.name.as_deref().unwrap_or(TERMINAL);
    let mut theme = if name == TERMINAL {
        Theme::terminal()
    } else {
        named(name, wanted)?
    };
    for (role, colour) in &choice.overrides {
        paint(&mut theme, role, colour)?;
    }
    if choice.transparent {
        theme.bg = None;
    }
    Ok(theme)
}

fn named(name: &str, wanted: Variant) -> Result<Theme, String> {
    let mut of_family = None;
    for (slug, document) in DOCUMENTS.iter() {
        let document = document.as_ref().map_err(Clone::clone)?;
        if *slug == name {
            return document.to_theme(slug);
        }
        if document.family == name && (of_family.is_none() || document.variant == wanted) {
            of_family = Some((*slug, document));
        }
    }
    match of_family {
        Some((slug, document)) => document.to_theme(slug),
        None => Err(format!("no theme is named {name:?}")),
    }
}

impl Document {
    fn to_theme(&self, slug: &str) -> Result<Theme, String> {
        let mut theme = Theme::terminal();
        for (role, value) in &self.roles {
            let painted = match value {
                toml::Value::String(colour) => paint(&mut theme, role, colour),
                toml::Value::Array(colours) => series(&mut theme, colours),
                _ => Err(format!("{role}: not a colour")),
            };
            painted.map_err(|error| format!("theme {slug}: {error}"))?;
        }
        Ok(theme)
    }
}

fn series(theme: &mut Theme, colours: &[toml::Value]) -> Result<(), String> {
    let parsed: Result<Vec<Color>, String> = colours
        .iter()
        .map(|value| value.as_str().ok_or("series: not a colour".to_owned()))
        .map(|colour| colour.and_then(parse))
        .collect();
    let parsed = parsed?;
    if parsed.len() < LEAST_SERIES {
        return Err(format!(
            "series: a chart needs {LEAST_SERIES} colours before it repeats one"
        ));
    }
    theme.series = parsed;
    Ok(())
}

/// Paints one role of `theme`. A ground spelled `default` is the
/// terminal's own, which is no fill at all.
fn paint(theme: &mut Theme, role: &str, colour: &str) -> Result<(), String> {
    let parsed = parse(colour).map_err(|error| format!("{role}: {error}"))?;
    let ground = (parsed != Color::Reset).then_some(parsed);
    match role {
        "bg" => theme.bg = ground,
        "selection_bg" => theme.selection_bg = ground,
        "stripe" => theme.stripe = ground,
        "fg" => theme.fg = parsed,
        "dim" => theme.dim = parsed,
        "border" => theme.border = parsed,
        "accent" => theme.accent = parsed,
        "over" => theme.over = parsed,
        "good" => theme.good = parsed,
        "caution" => theme.caution = parsed,
        _ => return Err(format!("{role}: not a role a theme has")),
    }
    Ok(())
}

fn parse(colour: &str) -> Result<Color, String> {
    if colour == DEFAULT_COLOUR {
        return Ok(Color::Reset);
    }
    Color::from_str(colour).map_err(|_| format!("{colour:?} is not a colour"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_theme_reads_and_names_enough_series_colours() {
        assert_eq!(listed().count(), EMBEDDED.len() + 1, "one failed to read");
        for (slug, _) in EMBEDDED {
            let theme = named(slug, Variant::Dark).unwrap_or_else(|error| panic!("{error}"));
            assert!(theme.bg.is_some(), "{slug} names a background");
            assert_ne!(theme.series(0), theme.series(LEAST_SERIES - 1), "{slug}");
        }
    }

    #[test]
    fn a_family_takes_the_variant_the_terminal_wants() {
        let light = named("gruvbox", Variant::Light).unwrap();
        let dark = named("gruvbox", Variant::Dark).unwrap();
        assert_eq!(light, named("gruvbox-light", Variant::Dark).unwrap());
        assert_eq!(dark, named("gruvbox-dark", Variant::Light).unwrap());
        let only = named("nord", Variant::Light).unwrap();
        assert_eq!(
            only,
            named("nord", Variant::Dark).unwrap(),
            "or the one it has"
        );
    }

    #[test]
    fn colorfgbg_names_a_light_ground_by_its_last_field() {
        assert_eq!(variant_of(None), Variant::Dark);
        assert_eq!(variant_of(Some("15;0")), Variant::Dark);
        assert_eq!(variant_of(Some("0;15")), Variant::Light);
        assert_eq!(variant_of(Some("0;default;7")), Variant::Light);
        assert_eq!(variant_of(Some("nonsense")), Variant::Dark);
    }

    #[test]
    fn a_choice_paints_over_the_theme_it_names() {
        let choice = Choice {
            name: Some("nord".to_owned()),
            transparent: true,
            overrides: BTreeMap::from([("accent".to_owned(), "#fabd2f".to_owned())]),
        };
        let theme = resolve(&choice, Variant::Dark).unwrap();
        assert_eq!(theme.accent, Color::Rgb(0xfa, 0xbd, 0x2f));
        assert_eq!(theme.bg, None, "transparent drops the background");
        assert!(theme.stripe.is_some(), "and keeps the rows' grounds");
    }

    #[test]
    fn what_does_not_read_says_where() {
        let unnamed = Choice {
            name: Some("gruvbocks".to_owned()),
            ..Choice::default()
        };
        assert!(
            resolve(&unnamed, Variant::Dark)
                .unwrap_err()
                .contains("gruvbocks")
        );
        let miscoloured = Choice {
            overrides: BTreeMap::from([("accent".to_owned(), "mauve-ish".to_owned())]),
            ..Choice::default()
        };
        let error = resolve(&miscoloured, Variant::Dark).unwrap_err();
        assert!(error.starts_with("accent:"), "{error}");
        let misroled = Choice {
            overrides: BTreeMap::from([("surface".to_owned(), "red".to_owned())]),
            ..Choice::default()
        };
        assert!(
            resolve(&misroled, Variant::Dark)
                .unwrap_err()
                .starts_with("surface:")
        );
    }
}
