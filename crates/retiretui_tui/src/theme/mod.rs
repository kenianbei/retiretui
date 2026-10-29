//! The roles a screen is coloured by, and the interaction theme plurimus
//! draws its widgets from. Widgets name a role, never a colour.

pub mod document;
pub mod ground;
pub mod library;
pub mod picker;
mod said;

use bevy_app::{App, PreStartup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Commands, IntoScheduleConfigs, Query, Res, ResMut, Resource};
use bevy_ecs::schedule::SystemSet;
use plurimus::core::ratatui_core::style::{Color, Modifier, Style};
use plurimus::core::{Background, TerminalCamera};
use plurimus::ui::UiTheme;
use plurimus::widgets::{TableStripe, WidgetSystems};

pub use said::{say, text};

use super::journal;
use super::settings::Settings;

pub fn plugin(app: &mut App) {
    app.add_plugins(picker::plugin);
    app.init_resource::<WantedVariant>();
    app.add_systems(PreStartup, wear_the_theme_set);
    app.configure_sets(Update, Repainted.before(WidgetSystems::Style));
    app.add_systems(Update, (sync_look, restripe).in_set(Repainted));
}

/// The variant a theme family is worn in: what the screen it is drawn on
/// is, light or dark.
#[derive(Resource, Clone, Copy, Default, Debug)]
pub struct WantedVariant(pub document::Variant);

/// What draws from the theme, run before the stock widgets restyle
/// themselves so a theme change reaches the frame it is made on.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Repainted;

/// The colours a screen names, each an RGB value: a theme decides every
/// colour, the terminal's palette none.
#[derive(Resource, Clone, PartialEq, Eq, Debug)]
pub struct Theme {
    /// The ground under everything.
    pub bg: Color,
    pub fg: Color,
    /// Metadata, axis labels, hints.
    pub dim: Color,
    /// A resting border.
    pub border: Color,
    /// The focused border, a pane's title, the cursor.
    pub accent: Color,
    /// The ground of a widget being pressed.
    pub selection_bg: Option<Color>,
    /// The alternate row's background.
    pub stripe: Option<Color>,
    /// A figure past its limit: an unfunded year, a failure.
    pub over: Color,
    /// A figure comfortably inside its limit.
    pub good: Color,
    /// A figure near its limit.
    pub caution: Color,
    series: Vec<Color>,
}

/// The default family in the dark variant.
#[cfg(test)]
impl Default for Theme {
    fn default() -> Self {
        library::default_theme(document::Variant::default())
    }
}

impl Theme {
    /// Every role in `ink`: what the default family's document is painted
    /// over, so that the family names every role or wears the ink.
    fn unpainted(ink: Color) -> Self {
        Self {
            bg: ink,
            fg: ink,
            dim: ink,
            border: ink,
            accent: ink,
            selection_bg: None,
            stripe: None,
            over: ink,
            good: ink,
            caution: ink,
            series: vec![ink],
        }
    }

    /// The colour `style` writes in, the theme's ink where it names none,
    /// as bevy's text takes it.
    #[must_use]
    pub fn ink(&self, style: Style) -> bevy_color::Color {
        style
            .fg
            .and_then(rgb)
            .or_else(|| rgb(self.fg))
            .unwrap_or_default()
    }

    /// The theme's ground as bevy takes it.
    #[must_use]
    pub fn ground(&self) -> bevy_color::Color {
        rgb(self.bg).unwrap_or_default()
    }

    #[must_use]
    pub fn dimmed(&self) -> Style {
        Style::new().fg(self.dim)
    }

    #[must_use]
    pub fn accented(&self) -> Style {
        Style::new().fg(self.accent)
    }

    #[must_use]
    pub fn bordered(&self) -> Style {
        Style::new().fg(self.border)
    }

    /// A frame's line: the accent while the keyboard is inside it, the
    /// border otherwise.
    #[must_use]
    pub fn edge(&self, is_lit: bool) -> Style {
        if is_lit {
            self.accented()
        } else {
            self.bordered()
        }
    }

    /// What holds the keyboard, or is chosen: bold in the accent.
    #[must_use]
    pub fn focused(&self) -> Style {
        self.accented().add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn exceeded(&self) -> Style {
        Style::new().fg(self.over)
    }

    /// The alternate row's patch; nothing where the theme names no stripe.
    #[must_use]
    pub fn striped(&self) -> Style {
        self.stripe
            .map_or_else(Style::new, |stripe| Style::new().bg(stripe))
    }

    /// The ground of a row left out of the banding, patched over a stripe
    /// that would fall on it.
    #[must_use]
    pub fn unbanded(&self) -> Style {
        Style::new().bg(self.bg)
    }

    /// The colour of a chart's `index`th dataset, wrapping past the last.
    ///
    /// # Panics
    ///
    /// Never for a constructed theme: every theme names a series colour.
    #[must_use]
    pub fn series(&self, index: usize) -> Color {
        self.series[index % self.series.len()]
    }

    /// The interaction theme every stock widget resolves its look from;
    /// the caret is the ground's colour on the accent.
    #[must_use]
    pub fn ui_theme(&self) -> UiTheme {
        let normal = Style::new().bg(self.bg).fg(self.fg);
        let hovered = self.stripe.map_or(normal, |stripe| normal.bg(stripe));
        let pressed = self
            .selection_bg
            .map_or(normal, |selection| normal.bg(selection));
        UiTheme::new()
            .with_normal(normal)
            .with_hovered(hovered)
            .with_pressed(pressed)
            .with_disabled(normal.fg(self.dim))
            .with_focused(self.focused())
            .with_caret(Style::new().fg(self.bg).bg(self.accent))
    }
}

/// A ground `colour` names as bevy takes it, and none where it names none,
/// so that what stands beneath shows through.
#[must_use]
pub fn paper(colour: Option<Color>) -> bevy_color::Color {
    colour.and_then(rgb).unwrap_or(bevy_color::Color::NONE)
}

fn rgb(colour: Color) -> Option<bevy_color::Color> {
    match colour {
        Color::Rgb(red, green, blue) => Some(bevy_color::Color::srgb_u8(red, green, blue)),
        _ => None,
    }
}

/// Reads the themes there are and puts on the one the settings name,
/// before anything draws from it. One that does not resolve is said so,
/// and what no name resolves to worn instead.
fn wear_the_theme_set(settings: Res<Settings>, wanted: Res<WantedVariant>, mut commands: Commands) {
    let themes = library::Themes::beside(&settings);
    let theme = themes
        .resolve(&settings.theme, wanted.0)
        .unwrap_or_else(|error| {
            journal::warn(format!("config.toml: {error}"));
            library::default_theme(wanted.0)
        });
    commands.insert_resource(theme);
    commands.insert_resource(themes);
}

fn sync_look(
    theme: Res<Theme>,
    mut stock: ResMut<UiTheme>,
    mut cameras: Query<&mut TerminalCamera>,
) {
    if !theme.is_changed() {
        return;
    }
    *stock = theme.ui_theme();
    for mut camera in &mut cameras {
        camera.background = Background::Clear(theme.bg);
    }
}

/// Bands every striped table in the theme's stripe, as it is spawned and
/// as the theme moves.
fn restripe(theme: Res<Theme>, mut stripes: Query<&mut TableStripe>) {
    for mut stripe in &mut stripes {
        if theme.is_changed() || stripe.is_added() {
            stripe.0 = theme.striped();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_caret_is_the_ground_on_the_accent() {
        let theme = Theme::default();
        let caret = theme.ui_theme().caret;
        assert_eq!((caret.bg, caret.fg), (Some(theme.accent), Some(theme.bg)));
    }

    #[test]
    fn a_themes_grounds_fill_the_states() {
        let theme = Theme {
            bg: Color::Rgb(0, 0, 0),
            stripe: Some(Color::Rgb(20, 20, 20)),
            selection_bg: Some(Color::Rgb(40, 40, 40)),
            ..Theme::default()
        };
        let ui = theme.ui_theme();
        assert_eq!(ui.normal.bg, Some(Color::Rgb(0, 0, 0)));
        assert_eq!(ui.hovered.bg, Some(Color::Rgb(20, 20, 20)));
        assert_eq!(ui.pressed.bg, Some(Color::Rgb(40, 40, 40)));
        let unstriped = Theme {
            stripe: None,
            selection_bg: None,
            ..theme
        };
        let ui = unstriped.ui_theme();
        assert_eq!((ui.hovered, ui.pressed), (ui.normal, ui.normal));
    }

    #[test]
    fn a_chart_past_the_colours_the_theme_names_starts_them_again() {
        let theme = Theme::default();
        assert_eq!(theme.series(theme.series.len()), theme.series(0));
        assert_ne!(theme.series(0), theme.series(1));
    }
}
