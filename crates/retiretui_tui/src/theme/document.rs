//! A theme as a document: the roles it names, the colour syntax, and what
//! a config paints over one.

use std::collections::BTreeMap;
use std::str::FromStr;

use plurimus::core::ratatui_core::style::Color;
use serde::Deserialize;

use super::Theme;

/// The name of the theme that is the terminal's own colours.
pub const TERMINAL: &str = "terminal";

/// What a colour is spelled as where it means the terminal's own.
const DEFAULT_COLOUR: &str = "default";
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

/// The theme document `text` holds.
///
/// # Errors
///
/// Where it is not a document, or a role or a colour of it does not read.
pub fn read(text: &str) -> Result<Painted, String> {
    let document: Document = toml::from_str(text).map_err(|error| error.message().to_owned())?;
    let mut theme = Theme::terminal();
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
        if self.transparent {
            theme.bg = None;
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
        assert!(unread("accent = \"mauve-ish\"").starts_with("accent:"));
        assert!(unread("surface = \"red\"").starts_with("surface:"));
        assert_eq!(unread("accent = 3"), "accent: not a colour");
        assert!(unread("series = [\"red\"]").starts_with("series:"));
        assert!(read("variant = \"dark\"").unwrap_err().contains("family"));
        assert_eq!(read(HEAD).unwrap().theme, Theme::terminal());
    }
}
