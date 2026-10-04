use std::time::Duration;

use bevy_time::TimeUpdateStrategy;
use plurimus::core::ratatui_core::style::Color;
use plurimus::term::KeyCode;

use super::*;
use crate::layout;
use crate::nav::Page;
use crate::sidebar::SIDEBAR_COLS;
use crate::support::{
    Headless, SIZE, cell_fg, cell_of, composed_frame, headless_app, headless_app_set, let_pass,
    press_key, said, scratch_plan, show, type_text,
};

const DIM: Play = Play::Dim(Color::Gray);
const BODY: Rect = Rect::new(0, 2, 60, 20);
const SLIDE: Play = Play::Exit {
    leaves: Leaves::Slide,
    within: BODY,
};
const DISSOLVE: Play = Play::Exit {
    leaves: Leaves::Dissolve,
    within: BODY,
};
const DIALOG: &str = "╭ Confirm";
const DRAWER: &str = "╭ Messages";

/// Where the accounts table's first item is drawn, outside any dialog:
/// the end of its balance, which no box centred on the body reaches.
/// A cell of the accounts table's first row, in from the sidebar.
const FIRST_ROW: (u16, u16) = (SIDEBAR_COLS + 6, layout::BODY_TOP + 2);
const WELL_PAST_ANY_EFFECT: Duration = Duration::from_secs(1);

/// One frame in which `by` passes, and every frame after it at that pace:
/// the real clock would decide how far an exit had got by the frame a
/// test looks at.
fn tick(app: &mut Headless, by: Duration) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(by));
    app.update();
}

fn moving_app(motion: Motion) -> Headless {
    let mut settings = Settings::default();
    settings.motion = motion;
    let mut app = headless_app_set(scratch_plan(), SIZE, settings);
    show(&mut app, Page::Accounts);
    app
}

#[test]
fn full_motion_gives_every_effect_its_own_length() {
    assert_eq!(Motion::Full.length(DIM), Duration::from_millis(120));
    assert_eq!(
        Motion::Full.length(Play::Coalesce),
        Duration::from_millis(150)
    );
    assert_eq!(
        Motion::Full.length(Play::Receipt(Color::Gray)),
        Duration::from_millis(400)
    );
    assert_eq!(Motion::Full.length(DISSOLVE), Duration::from_millis(150));
    assert_eq!(Motion::Full.length(SLIDE), Duration::from_millis(180));
}

#[test]
fn reduced_motion_keeps_what_informs_and_cuts_what_transitions() {
    assert_eq!(Motion::Reduced.length(DIM), Motion::Full.length(DIM));
    assert!(!Motion::Reduced.length(Play::Receipt(Color::Gray)).is_zero());
    assert!(Motion::Reduced.length(Play::Coalesce).is_zero());
    assert!(Motion::Reduced.length(DISSOLVE).is_zero());
    assert!(Motion::Reduced.length(SLIDE).is_zero());
}

#[test]
fn no_motion_gives_every_effect_no_time_at_all() {
    for play in [
        DIM,
        Play::Receipt(Color::Gray),
        Play::Coalesce,
        DISSOLVE,
        SLIDE,
    ] {
        assert!(Motion::Off.length(play).is_zero(), "{play:?}");
    }
}

#[test]
fn the_backdrop_dims_under_a_dialog_and_comes_back_when_it_closes() {
    let mut app = moving_app(Motion::Full);
    let resting = cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1);
    assert_ne!(resting, Some(Color::DarkGray));
    press_key(&mut app, KeyCode::Down);
    let resting_unfocused = cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1);

    press_key(&mut app, KeyCode::Char('d'));
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    let frame = composed_frame(&app);
    assert_eq!(
        cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1),
        Some(Color::DarkGray),
        "the page behind is dimmed: {frame}"
    );
    let (row, text) = frame
        .lines()
        .enumerate()
        .find(|(_, text)| text.contains("Delete"))
        .unwrap();
    let column = text.chars().position(|symbol| symbol == 'D').unwrap();
    assert_ne!(
        cell_fg(&app, column as u16, row as u16),
        Some(Color::DarkGray),
        "the dialog is not"
    );
    assert_eq!(
        cell_fg(&app, 1, SIZE.rows - 1),
        Some(Color::Cyan),
        "nor the hint row, which names its keys"
    );

    press_key(&mut app, KeyCode::Esc);
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    assert_eq!(cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1), resting_unfocused);
}

#[test]
fn with_motion_off_nothing_dims() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('d'));
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    assert!(composed_frame(&app).contains("Delete"));
    assert_ne!(
        cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1),
        Some(Color::DarkGray)
    );
}

#[test]
fn an_applied_row_fades_in_from_dim() {
    let mut app = moving_app(Motion::Full);
    press_key(&mut app, KeyCode::Enter);
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "x");
    press_key(&mut app, KeyCode::Enter);
    assert!(said(&app).is_empty(), "applied: {:?}", said(&app));
    assert_eq!(
        cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1),
        Some(Color::DarkGray),
        "the row starts from dim: {}",
        composed_frame(&app)
    );
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    assert_eq!(
        cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1),
        Some(Color::Cyan),
        "and settles into the cursor row's own accent"
    );
}

#[test]
fn the_motion_command_steps_through_the_three_and_says_which() {
    let mut app = headless_app(SIZE);
    assert_eq!(app.world().resource::<Settings>().motion, Motion::Off);
    for wanted in [Motion::Full, Motion::Reduced, Motion::Off] {
        press_key(&mut app, KeyCode::Char(':'));
        type_text(&mut app, "motion");
        press_key(&mut app, KeyCode::Enter);
        assert_eq!(app.world().resource::<Settings>().motion, wanted);
    }
    assert_eq!(said(&app), ["motion full", "motion reduced", "motion off"]);
}

#[test]
fn the_backdrop_spares_the_overlay_standing_deepest() {
    use bevy_ecs::prelude::World;
    use bevy_ecs::system::RunSystemOnce;

    use crate::overlay::Band;

    let upper = Rect::new(10, 5, 20, 8);
    let mut world = World::new();
    world.insert_resource(Theme::terminal());
    world.init_resource::<Cues>();
    world.spawn((ModalOpen, ComputedWidgetArea(upper), Band::at(1)));
    world.spawn((
        ModalOpen,
        ComputedWidgetArea(Rect::new(0, 0, 60, 20)),
        Band::at(0),
    ));
    world.run_system_once(dim_backdrop).unwrap();
    let cued = &world.resource::<Cues>().0;
    assert!(
        matches!(cued.as_slice(), [Cue::Play { area, .. }] if *area == upper),
        "{cued:?}"
    );
}

#[test]
fn a_closed_dialog_breaks_up_over_a_page_already_back() {
    let mut app = moving_app(Motion::Full);
    press_key(&mut app, KeyCode::Down);
    let resting = cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1);
    press_key(&mut app, KeyCode::Char('d'));
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    let stood = cell_of(&app, DIALOG);

    tick(&mut app, Duration::ZERO);
    press_key(&mut app, KeyCode::Esc);
    let whole = composed_frame(&app);
    assert_eq!(cell_of(&app, DIALOG), stood, "still drawn where it stood");
    assert_eq!(
        cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1),
        resting,
        "over a page no longer dimmed: {whole}"
    );

    tick(&mut app, Duration::from_millis(75));
    let breaking = composed_frame(&app);
    tick(&mut app, WELL_PAST_ANY_EFFECT);
    let gone = composed_frame(&app);
    assert!(!gone.contains("Confirm"), "{gone}");
    assert_ne!(breaking, whole, "some of it has given way");
    assert_ne!(breaking, gone, "and some has not");
}

#[test]
fn a_closed_panel_slides_out_through_the_bottom_of_the_body() {
    let mut app = moving_app(Motion::Full);
    press_key(&mut app, KeyCode::Char('m'));
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    let stood = cell_of(&app, DRAWER);

    tick(&mut app, Duration::ZERO);
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(cell_of(&app, DRAWER), stood, "still drawn where it stood");

    tick(&mut app, Duration::from_millis(60));
    let sliding = composed_frame(&app);
    assert!(
        cell_of(&app, DRAWER).1 > stood.1,
        "it has dropped: {sliding}"
    );
    tick(&mut app, WELL_PAST_ANY_EFFECT);
    let gone = composed_frame(&app);
    assert!(!gone.contains("Messages"), "{gone}");
    assert_eq!(
        sliding.lines().last(),
        gone.lines().last(),
        "the hint row beneath the body is never drawn over"
    );
}

#[test]
fn a_dialog_over_an_open_item_leaves_and_the_item_stays() {
    let mut app = moving_app(Motion::Full);
    press_key(&mut app, KeyCode::Enter);
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "x");
    press_key(&mut app, KeyCode::Esc);
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    let stood = cell_of(&app, DIALOG);

    tick(&mut app, Duration::ZERO);
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(cell_of(&app, DIALOG), stood, "still drawn where it stood");
    tick(&mut app, WELL_PAST_ANY_EFFECT);
    let gone = composed_frame(&app);
    assert!(!gone.contains("Confirm"), "{gone}");
    assert!(gone.contains("╭ Edit"), "{gone}");
}

#[test]
fn with_motion_reduced_or_off_what_closes_is_gone_at_once() {
    for motion in [Motion::Reduced, Motion::Off] {
        let mut app = moving_app(motion);
        press_key(&mut app, KeyCode::Down);
        press_key(&mut app, KeyCode::Char('d'));
        let_pass(&mut app, WELL_PAST_ANY_EFFECT);
        assert!(composed_frame(&app).contains(DIALOG));

        tick(&mut app, Duration::ZERO);
        press_key(&mut app, KeyCode::Esc);
        let closed = composed_frame(&app);
        assert!(!closed.contains("Confirm"), "{motion:?}: {closed}");
    }
}

mod standing {
    use bevy_ecs::prelude::{Commands, Component, World};
    use bevy_ecs::system::RunSystemOnce;
    use bevy_input_focus::InputFocus;
    use plurimus::bui::ComputedNodeRect;

    use super::*;
    use crate::layout::Body;
    use crate::overlay::{Focus, Standing};

    #[derive(Component, Default)]
    struct Panel;

    const PANEL: Rect = Rect::new(0, 14, 60, 8);

    fn close(mut standing: Standing<Panel>, mut commands: Commands) {
        standing.close(&mut commands);
    }

    fn reopen(mut standing: Standing<Panel>, mut commands: Commands) {
        standing.open(&mut commands);
    }

    fn world_with(panel: impl bevy_ecs::bundle::Bundle) -> World {
        let mut world = World::new();
        world.init_resource::<Cues>();
        world.init_resource::<Focus>();
        world.init_resource::<InputFocus>();
        let mut laid_out = ComputedNodeRect::default();
        laid_out.visible = BODY;
        world.spawn((Body, laid_out));
        world.spawn((Panel, panel));
        world
    }

    #[test]
    fn what_closes_is_cued_to_leave_as_its_kind_does() {
        let mut world = world_with((ComputedWidgetArea(PANEL), Leaves::Slide));
        world.run_system_once(close).unwrap();
        let slide = Cue::Play {
            play: SLIDE,
            area: PANEL,
            spared: Rect::ZERO,
        };
        assert_eq!(world.resource::<Cues>().0, [slide]);

        let mut world = world_with(ComputedWidgetArea(PANEL));
        world.run_system_once(close).unwrap();
        let cued = &world.resource::<Cues>().0;
        assert!(
            matches!(cued.as_slice(), [Cue::Play { play, .. }] if *play == DISSOLVE),
            "a box breaks up: {cued:?}"
        );
    }

    #[test]
    fn what_is_replaced_by_another_of_its_kind_does_not_leave_under_it() {
        let mut world = world_with((ComputedWidgetArea(PANEL), Leaves::Slide));
        world.run_system_once(reopen).unwrap();
        assert!(world.resource::<Cues>().0.is_empty());
        let standing = world.query::<&Panel>().iter(&world).count();
        assert_eq!(standing, 1, "the one it took down is gone");
    }

    #[test]
    fn what_closes_before_it_was_laid_out_has_nothing_to_leave_from() {
        let mut world = world_with(());
        world.run_system_once(close).unwrap();
        assert!(world.resource::<Cues>().0.is_empty());
        assert_eq!(world.query::<&Panel>().iter(&world).count(), 0);
    }
}
