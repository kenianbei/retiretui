//! The render side: cues become effects, played over the composed frame.

use std::collections::BTreeMap;

use bevy_app::App;
use bevy_ecs::prelude::{Commands, IntoScheduleConfigs, NonSendMut, ResMut, Resource};
use bevy_time::{Real, Time};
use plurimus::core::ratatui_core::layout::Rect;
use plurimus::core::{
    CompositeSystems, FrameBuffer, MainWorld, TerminalRenderApp, TerminalRenderAppExt,
    TerminalRenderSystems,
};
use tachyonfx::{CellFilter, Effect, EffectTimer, Interpolation, fx};

use super::{Cue, Cues, Key, Motion, Play};
use crate::commands::tui::settings::Settings;
use crate::commands::tui::theme::ground;

const EASING: Interpolation = Interpolation::QuadOut;

/// What the main world cued this frame, and how long the frame took.
#[derive(Resource, Default, Debug)]
struct Cued {
    cues: Vec<Cue>,
    motion: Motion,
    delta: std::time::Duration,
}

struct Running {
    play: Play,
    effect: Effect,
    is_arriving: bool,
}

/// The effects in play. An `Effect` is `Send` and not `Sync`, so they are
/// held outside the ordinary resources.
#[derive(Default)]
struct Playing(BTreeMap<Key, Running>);

pub fn install(app: &mut App) {
    app.add_extract_systems(extract);
    app.sub_app_mut(TerminalRenderApp)
        .init_resource::<Cued>()
        .world_mut()
        .insert_non_send(Playing::default());
    app.add_terminal_systems(
        TerminalRenderSystems::Composite,
        // After the theme's ground, so an effect eases between the colours
        // a cell is drawn in rather than from the terminal's default.
        play.in_set(CompositeSystems::PostProcess)
            .after(ground::Grounded),
    );
}

fn extract(mut main_world: ResMut<MainWorld>, mut commands: Commands) {
    let delta = main_world.resource::<Time<Real>>().delta();
    let motion = main_world.resource::<Settings>().motion;
    let cues = std::mem::take(&mut main_world.resource_mut::<Cues>().0);
    commands.insert_resource(Cued {
        cues,
        motion,
        delta,
    });
}

fn play(
    mut frame: ResMut<FrameBuffer>,
    mut extracted: ResMut<Cued>,
    mut playing: NonSendMut<Playing>,
) {
    let cues = std::mem::take(&mut extracted.cues);
    if extracted.motion == Motion::Off {
        playing.0.clear();
        return;
    }
    for cue in cues {
        start(&mut playing, cue, extracted.motion, *frame.0.area());
    }
    let tick = tachyonfx::Duration::from(extracted.delta);
    let whole = *frame.0.area();
    playing.0.retain(|_, running| {
        // An effect's first frame is its start, not a step along it: the
        // frame that cued it may have been a long one.
        let step = if std::mem::take(&mut running.is_arriving) {
            tachyonfx::Duration::ZERO
        } else {
            tick
        };
        running.effect.process(step, &mut frame.0, whole);
        !running.effect.done()
    });
}

fn start(playing: &mut Playing, cue: Cue, motion: Motion, frame: Rect) {
    let Cue::Play { play, area, spared } = cue else {
        if let Cue::Stop(gone) = cue {
            playing.0.remove(&gone);
        }
        return;
    };
    let key = play.key();
    let area = area.intersection(frame);
    let length = motion.length(play);
    if area.is_empty() || length.is_zero() {
        playing.0.remove(&key);
        return;
    }
    // A held effect cued again where it stands is moved, not restarted: the
    // backdrop does not fade in a second time as a dialog is resized.
    if let Some(running) = playing.0.get_mut(&key)
        && running.play == play
        && play.is_held()
    {
        running.effect.filter(outside(area, spared));
        return;
    }
    let millis = u32::try_from(length.as_millis()).unwrap_or(u32::MAX);
    let timer = EffectTimer::from_ms(millis, EASING);
    playing.0.insert(
        key,
        Running {
            play,
            effect: effect(play, area, spared, timer),
            is_arriving: true,
        },
    );
}

fn outside(area: Rect, spared: Rect) -> CellFilter {
    CellFilter::NoneOf(vec![CellFilter::Area(area), CellFilter::Area(spared)])
}

fn effect(play: Play, area: Rect, spared: Rect, timer: EffectTimer) -> Effect {
    match play {
        Play::Dim(colour) => {
            fx::never_complete(fx::fade_to_fg(colour, timer)).with_filter(outside(area, spared))
        }
        Play::Receipt(colour) => fx::fade_from_fg(colour, timer).with_area(area),
        Play::Coalesce => fx::coalesce(timer).with_area(area),
    }
}
