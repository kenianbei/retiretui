//! The planner in a window: the `retiretui-desktop` binary's arguments and
//! the launch it makes of them, and the skin that draws the shared tree
//! through bevy's own renderer on a grid of cells one monospace glyph
//! each.

mod areas;
mod chrome;
mod overlays;
mod widgets;

use anyhow::bail;
use bevy_a11y::AccessibilityPlugin;
use bevy_app::{
    App, AppExit, PanicHandlerPlugin, PostUpdate, PreUpdate, Startup, TaskPoolPlugin, Update,
};
use bevy_asset::{AssetPlugin, Assets, Handle};
use bevy_camera::{Camera2d, CameraPlugin, ClearColor};
use bevy_core_pipeline::CorePipelinePlugin;
use bevy_diagnostic::FrameCountPlugin;
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{
    Added, Changed, Commands, IntoScheduleConfigs, Or, Query, Res, ResMut, Single, With,
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
use bevy_winit::WinitPlugin;
use clap::Args;
use plurimus::bui::bevy_picking::DefaultPickingPlugins;
use plurimus::bui::bevy_ui_widgets::UiWidgetsPlugins;
use plurimus::core::{CameraSystems, CorePlugin, TerminalSize};
use plurimus::ui::bevy_input_focus::tab_navigation::TabNavigationPlugin;
use plurimus::ui::bevy_input_focus::{InputDispatchPlugin, InputFocusPlugin};
use retiretui_client::environment::state_dir;

use crate::TuiArgs;
use crate::command::{self, defer_named};
use crate::guard::MIN_SIZE;
use crate::theme::Theme;

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
    let floor = Vec2::new(f32::from(MIN_SIZE.cols), f32::from(MIN_SIZE.rows)) * CELL;
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
        chrome::plugin,
        widgets::plugin,
        overlays::plugin,
    ));
    app.add_systems(Startup, spawn_camera);
    app.add_systems(
        PreUpdate,
        (size_in_cells.before(CameraSystems::SyncSize), ask_on_close),
    );
    app.add_systems(Update, clear_to_the_ground);
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

/// Every text and span is set at the one size, whatever font it was given
/// for its weight or style; a size set back to bevy's own is pinned again.
fn pin_fonts(mut fonts: Query<&mut TextFont, Changed<TextFont>>) {
    for mut font in &mut fonts {
        if !matches!(font.font_size, FontSize::Px(size) if size == FONT_SIZE) {
            font.font_size = FontSize::Px(FONT_SIZE);
        }
    }
}

/// Every line is a row tall.
fn pin_line_heights(mut heights: Query<&mut LineHeight, Changed<LineHeight>>) {
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
