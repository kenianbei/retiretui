//! The planner in a window: the `retiretui-desktop` binary's arguments and
//! the launch it makes of them, and the skin that draws the shared tree
//! through bevy's own renderer on a grid of cells one monospace glyph
//! each.

mod areas;
mod bars;
mod basis;
mod browse;
mod card;
mod charts;
mod chrome;
mod fields;
mod material;
mod menus;
mod overlays;
mod overview;
mod pill;
mod tabs;
mod widgets;

use std::time::Duration;

use anyhow::bail;
use bevy_a11y::AccessibilityPlugin;
use bevy_app::{
    App, AppExit, Last, PanicHandlerPlugin, PostUpdate, PreUpdate, Startup, TaskPoolPlugin, Update,
};
use bevy_asset::{AssetPlugin, Assets, Handle};
use bevy_camera::{Camera2d, CameraPlugin, ClearColor};
use bevy_core_pipeline::CorePipelinePlugin;
use bevy_diagnostic::FrameCountPlugin;
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{
    Added, Changed, Commands, Component, IntoScheduleConfigs, Or, Query, Res, ResMut, Single, With,
    Without,
};
use bevy_image::ImagePlugin;
use bevy_input::InputPlugin;
use bevy_math::Vec2;
use bevy_mesh::MeshPlugin;
use bevy_render::RenderPlugin;
use bevy_sprite_render::SpriteRenderPlugin;
use bevy_text::{EmSize, Font, FontSize, LineHeight, RemSize, TextFont, TextPlugin};
use bevy_time::TimePlugin;
use bevy_transform::TransformPlugin;
use bevy_ui::{Node, UiPlugin, UiSystems};
use bevy_ui_render::UiRenderPlugin;
use bevy_window::{
    PrimaryWindow, Window, WindowCloseRequested, WindowPlugin, WindowResizeConstraints,
};
use bevy_winit::{UpdateMode, WinitPlugin, WinitSettings};
use clap::Args;
use plurimus::bui::bevy_picking::DefaultPickingPlugins;
use plurimus::bui::bevy_ui_widgets::UiWidgetsPlugins;
use plurimus::core::{CameraSystems, CorePlugin, TerminalSize};
use plurimus::ui::bevy_input_focus::tab_navigation::TabNavigationPlugin;
use plurimus::ui::bevy_input_focus::{InputDispatchPlugin, InputFocusPlugin};
use retiretui_client::environment::state_dir;

use crate::TuiArgs;
use crate::command::{self, defer_named};
use crate::theme::Theme;
use crate::tools::is_any_live;
use crate::watch::POLL_SECONDS;

/// Arguments of the desktop binary: the `tui` command's, and the light
/// variant.
#[derive(Args, Debug)]
pub struct DesktopArgs {
    /// What to open, and where else the tax tables are.
    #[command(flatten)]
    pub tui: TuiArgs,
    /// Wear a theme family's light variant.
    #[arg(long)]
    pub light: bool,
}

const TITLE: &str = "RetireTui";
const LOG_FILE: &str = "desktop.log";
/// The face every glyph is drawn in, embedded so a window needs no font
/// of the machine's; its licence is beside it.
const FACE: &[u8] = include_bytes!("fonts/Inconsolata-Regular.ttf");
const FONT_SIZE: f32 = 16.0;
/// A cell: the face advances half its size, and a row leaves a quarter
/// of the size between lines; nothing measures the face at runtime.
const CELL: Vec2 = Vec2::new(FONT_SIZE / 2.0, FONT_SIZE * 1.25);

/// A figure the window says at twice the size, which the pins leave as
/// said: a glyph is then two cells wide and two tall, so it stays on the
/// grid. Only on text the window spawns itself, never on a shared node.
#[derive(Component, Debug, Default)]
pub(super) struct Figure;

/// Runs the planner in a window until it is quit.
///
/// # Errors
///
/// Where the tables, the history or the log cannot be set up, the session
/// cannot open, or the app exits with an error.
pub fn run(args: &DesktopArgs) -> anyhow::Result<()> {
    let launch = crate::native_launch(&args.tui, args.light)?;
    let mut app = App::new();
    add_window(&mut app);
    crate::build_shared(&mut app, launch, skin).map_err(anyhow::Error::msg)?;
    crate::install_log(&app, state_dir().map(|dir| dir.join(LOG_FILE)))?;
    match app.run() {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => bail!("desktop exited with error code {code}"),
    }
}

/// Bevy's window, renderer, text and picking from the granular crates, as
/// plurimus's own windowed example assembles them, with plurimus's core
/// beneath so the shell's focus, overlays and scrolling keep their
/// meaning; nothing of bevy's logging, since the journal is the process's
/// subscriber. The window comes first: plurimus spawns a virtual one only
/// where none is there by then.
fn add_window(app: &mut App) {
    let (cols, rows) = chrome::floor_cells();
    let floor = Vec2::new(f32::from(cols), f32::from(rows)) * CELL;
    app.add_plugins((
        PanicHandlerPlugin,
        TaskPoolPlugin::default(),
        FrameCountPlugin,
        TimePlugin,
        TransformPlugin,
        InputPlugin,
        InputFocusPlugin,
        InputDispatchPlugin,
        TabNavigationPlugin,
        WindowPlugin {
            primary_window: Some(Window {
                title: TITLE.into(),
                resolution: floor.as_uvec2().into(),
                resize_constraints: WindowResizeConstraints {
                    min_width: floor.x,
                    min_height: floor.y,
                    ..WindowResizeConstraints::default()
                },
                ..Window::default()
            }),
            close_when_requested: false,
            ..WindowPlugin::default()
        },
        AccessibilityPlugin,
        AssetPlugin::default(),
        WinitPlugin::default(),
    ));
    app.add_plugins((
        RenderPlugin::default(),
        ImagePlugin::default(),
        MeshPlugin,
        CameraPlugin,
        CorePipelinePlugin,
        SpriteRenderPlugin,
        TextPlugin,
        UiPlugin,
        UiRenderPlugin,
        DefaultPickingPlugins,
        UiWidgetsPlugins,
        CorePlugin,
    ));
}

/// The window's look over the shared tree: one cell size every `Rem` and
/// `Em` resolve to, the face every text is set in, the theme's ground,
/// and the frame's size in cells for what lays itself out by it.
fn skin(app: &mut App) {
    app.world_mut()
        .resource_mut::<Assets<Font>>()
        .insert(&Handle::<Font>::default(), Font::from_bytes(FACE.to_vec()))
        .expect("the default font handle is live");
    app.insert_resource(RemSize(CELL.y));
    app.add_plugins((
        areas::plugin,
        bars::plugin,
        basis::plugin,
        browse::plugin,
        charts::plugin,
        card::plugin,
        chrome::plugin,
        tabs::plugin,
        widgets::plugin,
        menus::plugin,
        overview::plugin,
        fields::plugin,
        material::plugin,
        overlays::plugin,
    ));
    app.add_systems(Startup, spawn_camera);
    app.add_systems(
        PreUpdate,
        (size_in_cells.before(CameraSystems::SyncSize), ask_on_close),
    );
    app.add_systems(Update, clear_to_the_ground);
    app.add_systems(Last, sleep_when_idle);
    app.add_systems(
        PostUpdate,
        (
            (pin_fonts, pin_line_heights).before(UiSystems::Content),
            size_ems.after(UiSystems::Content).before(UiSystems::Layout),
        ),
    );
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// How often the window draws: every frame while work runs beside the
/// frames, so its progress and its answer show as they happen; otherwise
/// for an input and once a beat, the file watch's, in and out of focus
/// alike, since the plan is edited elsewhere while the window is not
/// focused. Last in the frame, so work spawned in it is drawn from the
/// next.
fn sleep_when_idle(mut settings: ResMut<WinitSettings>) {
    let wanted = if is_any_live() {
        UpdateMode::Continuous
    } else {
        UpdateMode::reactive_low_power(Duration::from_secs_f32(POLL_SECONDS))
    };
    if settings.focused_mode != wanted || settings.unfocused_mode != wanted {
        *settings = WinitSettings {
            focused_mode: wanted,
            unfocused_mode: wanted,
        };
    }
}

/// What the shell lays itself out by: the window in whole cells.
fn size_in_cells(window: Single<&Window, With<PrimaryWindow>>, mut size: ResMut<TerminalSize>) {
    let whole = (window.resolution.size() / CELL).floor();
    size.set_if_neq(TerminalSize::new(whole.x as u16, whole.y as u16));
}

/// Every node's em is a cell's width, where bevy would make it the font
/// size of a text node and leave the rest at its default.
fn size_ems(mut nodes: Query<&mut EmSize, (With<Node>, Or<(Added<Node>, Changed<EmSize>)>)>) {
    for mut em in &mut nodes {
        em.set_if_neq(EmSize(CELL.x));
    }
}

/// Every text and span but a figure is set at the one size, whatever font
/// it was given for its weight or style; a size set back to bevy's own is
/// pinned again.
fn pin_fonts(mut fonts: Query<&mut TextFont, (Changed<TextFont>, Without<Figure>)>) {
    for mut font in &mut fonts {
        if !matches!(font.font_size, FontSize::Px(size) if size == FONT_SIZE) {
            font.font_size = FontSize::Px(FONT_SIZE);
        }
    }
}

/// Every line but a figure's is a row tall.
fn pin_line_heights(mut heights: Query<&mut LineHeight, (Changed<LineHeight>, Without<Figure>)>) {
    for mut height in &mut heights {
        height.set_if_neq(LineHeight::Px(CELL.y));
    }
}

fn clear_to_the_ground(theme: Res<Theme>, mut clear: ResMut<ClearColor>) {
    if theme.is_changed() {
        clear.0 = theme.ground();
    }
}

/// The window's close button asks what the `quit` command asks, so an
/// unsaved draft is not lost to it.
fn ask_on_close(mut closing: MessageReader<WindowCloseRequested>, mut commands: Commands) {
    if closing.read().next().is_some() {
        defer_named(&mut commands, command::QUIT);
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::hierarchy::ChildOf;
    use bevy_ecs::prelude::{Entity, With, Without, World};
    use bevy_ecs::system::RunSystemOnce;
    use plurimus::core::UiWidget;
    use plurimus::widgets::{Button, ListBox};

    use super::*;
    use crate::chart::SeriesChart;
    use crate::layout::Rule;
    use crate::nav::{Page, SurfaceRoot};
    use crate::pane::Framed;
    use crate::support::{SIZE, headless_app, show};
    use crate::tools::Keyed;

    /// The page a widget stands on, climbing to its surface's root.
    fn page_of(world: &World, mut entity: Entity) -> Option<Page> {
        loop {
            if let Some(SurfaceRoot(surface)) = world.get::<SurfaceRoot>(entity) {
                return *surface;
            }
            entity = world.get::<ChildOf>(entity)?.parent();
        }
    }

    fn modes(app: &App) -> (UpdateMode, UpdateMode) {
        let settings = app.world().resource::<WinitSettings>();
        (settings.focused_mode, settings.unfocused_mode)
    }

    /// The count of work is the process's, so the sleeping edge is
    /// asserted only while no other test holds work of its own.
    #[test]
    fn a_figure_keeps_its_size_through_the_pins_and_the_rest_is_pinned() {
        let mut world = World::new();
        let doubled = FONT_SIZE * 2.0;
        let figure = world
            .spawn((Figure, TextFont::from_font_size(doubled)))
            .id();
        let plain = world.spawn(TextFont::from_font_size(doubled)).id();
        world.run_system_once(pin_fonts).unwrap();
        let size = |entity: Entity| world.get::<TextFont>(entity).unwrap().font_size;
        assert_eq!(size(figure), FontSize::Px(doubled));
        assert_eq!(size(plain), FontSize::Px(FONT_SIZE));
    }

    #[test]
    fn the_window_draws_every_frame_while_work_runs_and_sleeps_otherwise() {
        let mut app = headless_app(SIZE);
        app.init_resource::<WinitSettings>();
        app.add_systems(Last, sleep_when_idle);
        let held = Keyed::spawn((), |_| ());
        app.update();
        let continuous = (UpdateMode::Continuous, UpdateMode::Continuous);
        assert_eq!(modes(&app), continuous, "work under way");
        drop(held);
        app.update();
        if !is_any_live() {
            let asleep = UpdateMode::reactive_low_power(Duration::from_secs_f32(POLL_SECONDS));
            assert_eq!(modes(&app), (asleep, asleep), "none");
        }
    }

    /// The window draws a pane's frame, a chart's plot, a rule, a list
    /// and a button itself and nothing else ratatui draws, so no page may
    /// hold another widget.
    #[test]
    fn every_page_s_widgets_are_what_the_window_draws() {
        let mut app = headless_app(SIZE);
        for page in Page::ALL {
            show(&mut app, page);
        }
        let world = app.world_mut();
        let mut widgets = world.query_filtered::<Entity, (
            With<UiWidget>,
            Without<Framed>,
            Without<SeriesChart>,
            Without<Rule>,
            Without<ListBox>,
            Without<Button>,
        )>();
        let strays: Vec<(Entity, Page)> = widgets
            .iter(world)
            .filter_map(|widget| page_of(world, widget).map(|page| (widget, page)))
            .collect();
        assert!(
            strays.is_empty(),
            "widgets the window does not draw: {strays:?}"
        );
    }
}
