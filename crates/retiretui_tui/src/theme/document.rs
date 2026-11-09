//! A theme as a document: the roles it names, the colour syntax, and what
//! a config paints over one.

use std::collections::BTreeMap;
use std::str::FromStr;

use plurimus::core::ratatui_core::style::Color;
use serde::Deserialize;

use super::Theme;
use super::library::default_theme;

pub const LEAST_SERIES: usize = 4;
const GROUND_VARIABLE: &str = "COLORFGBG";
const GROUND_SEPARATOR: char = ';';
/// The backgrounds a terminal names that are dark; the rest are light.
const DARK_GROUNDS: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 8];

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    #[default]
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
    /// A theme, or a family of them; the default family where unnamed.
    #[serde(default)]
    pub name: Option<String>,
    /// Any role, painted over the named theme.
    #[serde(flatten)]
    pub overrides: BTreeMap<String, String>,
}

/// The variant `COLORFGBG` says the terminal wants, and dark where it says
/// nothing.
pub fn terminal_variant() -> Variant {
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

/// A theme document that read.
#[derive(Debug)]
pub struct Painted {
    pub family: String,
    pub variant: Variant,
    pub theme: Theme,
}

/// The theme document `text` holds, its roles painted over the default
/// family in the document's variant, so a role it leaves unstated takes
/// the default's.
///
/// # Errors
///
/// Where it is not a document, or a role or a colour of it does not read.
pub fn read(text: &str) -> Result<Painted, String> {
    read_over(text, default_theme)
}

/// `text` painted over what `base` gives for its variant.
pub(super) fn read_over(
    text: &str,
    base: impl FnOnce(Variant) -> Theme,
) -> Result<Painted, String> {
    let document: Document = toml::from_str(text).map_err(|error| error.message().to_owned())?;
    let mut theme = base(document.variant);
    for (role, value) in &document.roles {
        match value {
            toml::Value::String(colour) => paint(&mut theme, role, colour)?,
            toml::Value::Array(colours) => series(&mut theme, colours)?,
            _ => return Err(format!("{role}: not a colour")),
        }
    }
    Ok(Painted {
        family: document.family,
        variant: document.variant,
        theme,
    })
}

impl Choice {
    /// `theme` as the choice paints it.
    ///
    /// # Errors
    ///
    /// Where it paints what is not a role, or in what is not a colour.
    pub fn over(&self, mut theme: Theme) -> Result<Theme, String> {
        for (role, colour) in &self.overrides {
            paint(&mut theme, role, colour)?;
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

fn paint(theme: &mut Theme, role: &str, colour: &str) -> Result<(), String> {
    let parsed = parse(colour).map_err(|error| format!("{role}: {error}"))?;
    match role {
        "bg" => theme.bg = parsed,
        "selection_bg" => theme.selection_bg = Some(parsed),
        "stripe" => theme.stripe = Some(parsed),
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

/// A colour as a theme spells one: `#rrggbb`, and nothing a terminal's
/// palette would decide.
fn parse(colour: &str) -> Result<Color, String> {
    match Color::from_str(colour) {
        Ok(rgb @ Color::Rgb(..)) => Ok(rgb),
        _ => Err(format!("{colour:?} is not spelled #rrggbb")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorfgbg_names_a_light_ground_by_its_last_field() {
        assert_eq!(variant_of(None), Variant::Dark);
        assert_eq!(variant_of(Some("15;0")), Variant::Dark);
        assert_eq!(variant_of(Some("0;15")), Variant::Light);
        assert_eq!(variant_of(Some("0;default;7")), Variant::Light);
        assert_eq!(variant_of(Some("nonsense")), Variant::Dark);
    }

    #[test]
    fn a_document_says_which_role_or_colour_does_not_read() {
        const HEAD: &str = "family = \"mine\"\nvariant = \"dark\"\n";
        let unread = |roles: &str| read(&format!("{HEAD}{roles}")).unwrap_err();
        assert!(unread("surface = \"red\"").starts_with("surface:"));
        assert_eq!(unread("accent = 3"), "accent: not a colour");
        assert!(unread("series = [\"#ff0000\"]").starts_with("series:"));
        assert!(read("variant = \"dark\"").unwrap_err().contains("family"));
    }

    #[test]
    fn a_colour_is_spelled_rgb_and_never_a_palette_entry() {
        const HEAD: &str = "family = \"mine\"\nvariant = \"dark\"\n";
        for named in ["red", "darkgray", "8", "default", "mauve-ish"] {
            let said = read(&format!("{HEAD}accent = \"{named}\"")).unwrap_err();
            assert_eq!(said, format!("accent: {named:?} is not spelled #rrggbb"));
        }
        let read = read(&format!("{HEAD}accent = \"#fabd2f\"")).unwrap();
        assert_eq!(read.theme.accent, Color::Rgb(0xfa, 0xbd, 0x2f));
    }

    #[test]
    fn a_role_left_unstated_takes_the_default_family_in_the_document_variant() {
        let dark = read("family = \"mine\"\nvariant = \"dark\"\n").unwrap();
        assert_eq!(dark.theme, default_theme(Variant::Dark));
        let light = read("family = \"mine\"\nvariant = \"light\"\naccent = \"#010203\"\n").unwrap();
        let expected = Theme {
            accent: Color::Rgb(1, 2, 3),
            ..default_theme(Variant::Light)
        };
        assert_eq!(light.theme, expected);
    }
}
