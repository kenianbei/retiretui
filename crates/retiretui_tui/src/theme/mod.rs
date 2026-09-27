//! The roles a screen is coloured by, and the interaction theme plurimus
//! draws its widgets from. Widgets name a role, never a colour.

pub mod document;
pub mod ground;
pub mod picker;

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{IntoScheduleConfigs, Query, Res, ResMut, Resource};
use bevy_ecs::schedule::SystemSet;
use plurimus::core::ratatui_core::style::{Color, Modifier, Style};
use plurimus::core::{Background, TerminalCamera};
use plurimus::ui::UiTheme;
use plurimus::widgets::{TableStripe, WidgetSystems};

use super::journal;
use super::settings::Settings;

pub fn plugin(app: &mut App) {
    app.add_plugins((picker::plugin, ground::plugin));
    app.insert_resource(Theme::terminal());
    app.add_systems(Startup, wear_the_theme_set);
    app.configure_sets(Update, Repainted.before(WidgetSystems::Style));
    app.add_systems(Update, (sync_look, restripe).in_set(Repainted));
}

/// What draws from the theme, run before the stock widgets restyle
/// themselves so a theme change reaches the frame it is made on.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Repainted;

/// The colours a screen names. A ground the theme leaves unset is the
/// terminal's own.
#[derive(Resource, Clone, PartialEq, Eq, Debug)]
pub struct Theme {
    pub bg: Option<Color>,
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

impl Theme {
    /// The terminal's own sixteen colours, its grounds left alone, so the
    /// app matches whatever the terminal is themed as.
    #[must_use]
    pub fn terminal() -> Self {
        Self {
            bg: None,
            fg: Color::Reset,
            dim: Color::DarkGray,
            border: Color::DarkGray,
            accent: Color::Cyan,
            selection_bg: None,
            stripe: None,
            over: Color::Red,
            good: Color::Green,
            caution: Color::Yellow,
            series: vec![
                Color::Cyan,
                Color::Green,
                Color::Magenta,
                Color::Yellow,
                Color::Blue,
                Color::Red,
            ],
        }
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

    /// The colour of a chart's `index`th dataset, wrapping past the last.
    ///
    /// # Panics
    ///
    /// Never for a constructed theme: every theme names a series colour.
    #[must_use]
    pub fn series(&self, index: usize) -> Color {
        self.series[index % self.series.len()]
    }

    fn background(&self) -> Background {
        self.bg
            .map_or(Background::TerminalDefault, Background::Clear)
    }

    /// The interaction theme every stock widget resolves its look from.
    #[must_use]
    pub fn ui_theme(&self) -> UiTheme {
        let normal = self
            .bg
            .map_or_else(Style::new, |bg| Style::new().bg(bg))
            .fg(self.fg);
        let hovered = self.stripe.map_or(normal, |stripe| normal.bg(stripe));
        let pressed = self
            .selection_bg
            .map_or(normal, |selection| normal.bg(selection));
        UiTheme::new()
            .with_normal(normal)
            .with_hovered(hovered)
            .with_pressed(pressed)
            .with_disabled(normal.fg(self.dim))
            .with_focused(Style::new().fg(self.accent).add_modifier(Modifier::BOLD))
            .with_caret(self.caret())
    }

    /// A text caret in the accent over the theme's ground, or the stock
    /// reversed cell where the ground is the terminal's own and no colour
    /// is sure to show against it.
    fn caret(&self) -> Style {
        self.bg.map_or_else(
            || Style::new().add_modifier(Modifier::REVERSED),
            |bg| Style::new().fg(bg).bg(self.accent),
        )
    }
}

/// Puts on the theme the settings name. One that does not resolve is said
/// so, and the terminal's own worn instead.
fn wear_the_theme_set(settings: Res<Settings>, mut theme: ResMut<Theme>) {
    match document::resolve(&settings.theme, document::wanted_variant()) {
        Ok(set) => *theme = set,
        Err(error) => journal::warn(format!("config.toml: {error}")),
    }
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
    let background = theme.background();
    for mut camera in &mut cameras {
        camera.background = background;
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
    fn a_theme_with_a_ground_draws_the_caret_in_its_accent() {
        let themed = Theme {
            bg: Some(Color::Black),
            ..Theme::terminal()
        };
        let caret = themed.ui_theme().caret;
        assert_eq!((caret.bg, caret.fg), (Some(themed.accent), themed.bg));
        let terminal = Theme::terminal().ui_theme().caret;
        assert!(terminal.add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn the_terminal_theme_leaves_the_grounds_to_the_terminal() {
        let theme = Theme::terminal();
        let ui = theme.ui_theme();
        assert_eq!(ui.normal, Style::new().fg(Color::Reset));
        assert_eq!(ui.hovered, ui.normal);
        assert_eq!(ui.pressed, ui.normal);
        assert_eq!(ui.focused.fg, Some(Color::Cyan));
        assert_eq!(theme.background(), Background::TerminalDefault);
    }

    #[test]
    fn a_theme_with_grounds_fills_the_states_from_them() {
        let theme = Theme {
            bg: Some(Color::Rgb(0, 0, 0)),
            stripe: Some(Color::Rgb(20, 20, 20)),
            selection_bg: Some(Color::Rgb(40, 40, 40)),
            ..Theme::terminal()
        };
        let ui = theme.ui_theme();
        assert_eq!(ui.normal.bg, Some(Color::Rgb(0, 0, 0)));
        assert_eq!(ui.hovered.bg, Some(Color::Rgb(20, 20, 20)));
        assert_eq!(ui.pressed.bg, Some(Color::Rgb(40, 40, 40)));
        assert_eq!(theme.background(), Background::Clear(Color::Rgb(0, 0, 0)));
    }

    #[test]
    fn a_chart_past_the_colours_the_theme_names_starts_them_again() {
        let theme = Theme::terminal();
        assert_eq!(theme.series(theme.series.len()), theme.series(0));
        assert_ne!(theme.series(0), theme.series(1));
    }
}
