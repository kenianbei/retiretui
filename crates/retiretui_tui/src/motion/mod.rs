//! Motion that says what just happened: the backdrop dimming under an
//! overlay, a flash on the row just applied, an overlay coalescing as it
//! arrives. Nothing continuous, nothing per keystroke.
//!
//! The effects run over the composed frame, which exists in the render
//! sub-app alone, so the main world queues a description of each - a
//! [`Cue`] - and the render side plays it.

mod render;

use std::time::Duration;

use bevy_app::{App, PostUpdate};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Local, Query, Res, ResMut, Resource, With,
};
use bevy_ui::UiSystems;
use plurimus::bui::ComputedNodeRect;
use plurimus::core::ratatui_core::layout::Rect;
use plurimus::core::ratatui_core::style::Color;
use plurimus::ui::{ComputedWidgetArea, ModalOpen};
use serde::Deserialize;

use super::command::Outcome;
use super::journal;
use super::layout::HintRow;
use super::overlay::Band;
use super::settings::Settings;
use super::theme::Theme;

pub fn plugin(app: &mut App) {
    app.init_resource::<Cues>();
    app.add_systems(
        PostUpdate,
        (dim_backdrop, cue_arrivals).after(UiSystems::PostLayout),
    );
    render::install(app);
}

const MOTION_KEY: [&str; 1] = ["motion"];

/// How much the shell moves. Every effect's length passes through it.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    #[default]
    Full,
    /// What says something happened, without what only eases a change in:
    /// the dim and the receipt, not the coalesce.
    Reduced,
    Off,
}

impl Motion {
    const fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Reduced => "reduced",
            Self::Off => "off",
        }
    }

    const fn next(self) -> Self {
        match self {
            Self::Full => Self::Reduced,
            Self::Reduced => Self::Off,
            Self::Off => Self::Full,
        }
    }

    /// How long `play` runs, which is no time at all where it is cut.
    #[must_use]
    pub const fn length(self, play: Play) -> Duration {
        match (self, play) {
            (Self::Off, _) | (Self::Reduced, Play::Coalesce) => Duration::ZERO,
            (Self::Full | Self::Reduced, _) => play.length(),
        }
    }
}

/// Which effect is running, so that cueing one again replaces it rather
/// than stacking a second over it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Key {
    Backdrop,
    Receipt,
    Arrival,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Play {
    /// Everything outside the area fades toward the colour, and is held
    /// there until stopped.
    Dim(Color),
    /// The area's text fades in from the colour.
    Receipt(Color),
    /// The area's cells settle into place.
    Coalesce,
}

impl Play {
    /// Each effect runs under a key of its own.
    const fn key(self) -> Key {
        match self {
            Self::Dim(_) => Key::Backdrop,
            Self::Receipt(_) => Key::Receipt,
            Self::Coalesce => Key::Arrival,
        }
    }

    const fn length(self) -> Duration {
        Duration::from_millis(match self {
            Self::Dim(_) => 120,
            Self::Receipt(_) => 400,
            Self::Coalesce => 150,
        })
    }

    const fn is_held(self) -> bool {
        matches!(self, Self::Dim(_))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cue {
    Play {
        play: Play,
        area: Rect,
        /// Left alone by an effect that plays outside its area.
        spared: Rect,
    },
    Stop(Key),
}

/// What has been cued since the render side last took it.
#[derive(Resource, Default, Debug)]
pub struct Cues(Vec<Cue>);

impl Cues {
    pub fn play(&mut self, play: Play, area: Rect) {
        self.0.push(Cue::Play {
            play,
            area,
            spared: Rect::ZERO,
        });
    }
}

/// Marks an overlay that has just opened, until it has an area to arrive
/// in.
#[derive(Component, Debug)]
pub struct Arriving;

/// The `motion` command: the next of full, reduced, and off, kept.
pub fn cycle(mut settings: ResMut<Settings>) -> Outcome {
    let motion = settings.motion.next();
    settings.motion = motion;
    match settings.keep(&MOTION_KEY, motion.name()) {
        Ok(()) => journal::say(format!("motion {}", motion.name())),
        Err(error) => journal::warn(format!(
            "motion {}, for this session only: {error}",
            motion.name()
        )),
    }
    Outcome::Done
}

/// Dims everything outside the topmost overlay while one stands, sparing
/// the hint row, which names the overlay's keys.
fn dim_backdrop(
    overlays: Query<(&ComputedWidgetArea, Option<&Band>), With<ModalOpen>>,
    hint_rows: Query<&ComputedNodeRect, With<HintRow>>,
    theme: Res<Theme>,
    mut cues: ResMut<Cues>,
    mut dimmed: Local<Option<(Rect, Rect)>>,
) {
    let topmost = overlays
        .iter()
        .filter(|(area, _)| !area.0.is_empty())
        .max_by_key(|(_, band)| band.copied())
        .map(|(area, _)| area.0);
    let spared = hint_rows.single().map_or(Rect::ZERO, |row| row.rect);
    let standing = topmost.map(|area| (area, spared));
    if standing == *dimmed && !theme.is_changed() {
        return;
    }
    *dimmed = standing;
    match standing {
        Some((area, spared)) => cues.0.push(Cue::Play {
            play: Play::Dim(theme.dim),
            area,
            spared,
        }),
        None => cues.0.push(Cue::Stop(Key::Backdrop)),
    }
}

fn cue_arrivals(
    arriving: Query<(Entity, &ComputedWidgetArea), With<Arriving>>,
    mut cues: ResMut<Cues>,
    mut commands: Commands,
) {
    for (entity, area) in &arriving {
        if area.0.is_empty() {
            continue;
        }
        cues.play(Play::Coalesce, area.0);
        commands.entity(entity).remove::<Arriving>();
    }
}

#[cfg(test)]
mod tests;
