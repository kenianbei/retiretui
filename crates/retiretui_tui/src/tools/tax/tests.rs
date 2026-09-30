use plurimus::term::KeyCode;
use retiretui_client::setup::examples::named;

use super::Tabled;
use crate::nav::Page;
use crate::session::YearCursor;
use crate::support::{
    Headless, SIZE, composed_frame, headless_app_at, picker_rows, press_key, redrawn, scratch_dir,
    show, type_text,
};

/// The example that lives in Oregon, then Washington from 2030.
fn moving_app() -> Headless {
    let (_, _, text) = named("moving-states.toml").expect("the example");
    let path = scratch_dir().join("moving.toml");
    std::fs::write(&path, text).unwrap();
    let mut app = headless_app_at(path, SIZE);
    show(&mut app, Page::TaxTables);
    app
}

fn titles(app: &bevy_app::App) -> Vec<String> {
    let tabled = app
        .world()
        .resource::<Tabled>()
        .0
        .as_ref()
        .expect("tables shown");
    tabled
        .sections
        .iter()
        .map(|section| section.title.clone())
        .collect()
}

fn picked(app: &mut bevy_app::App, key: char, query: &str) {
    press_key(app, KeyCode::Char(key));
    type_text(app, query);
    press_key(app, KeyCode::Enter);
}

#[test]
fn the_tables_follow_the_year_and_the_picks() {
    let mut app = moving_app();
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("Tax Tables · 2026 · Married filing jointly · Oregon"),
        "{frame}"
    );
    assert!(titles(&app).contains(&"Oregon income tax".to_owned()));
    press_key(&mut app, KeyCode::Char(']'));
    assert_eq!(
        app.world().resource::<YearCursor>().0,
        Some(2027),
        "the shared year moves"
    );
    press_key(&mut app, KeyCode::Char('t'));
    let rows = picker_rows(&mut app);
    let own = "Where the plan lives (Oregon)";
    assert!(rows.iter().any(|(label, _)| label == own), "{rows:?}");
    type_text(&mut app, "texas");
    press_key(&mut app, KeyCode::Enter);
    picked(&mut app, 'f', "single");
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("Tax Tables · 2027 · Single · Texas"),
        "{frame}"
    );
    assert!(titles(&app).contains(&"Texas income tax".to_owned()));
    press_key(&mut app, KeyCode::Char('t'));
    type_text(&mut app, "washington");
    redrawn(&mut app);
    assert!(
        titles(&app).contains(&"Washington brackets".to_owned()),
        "tried on"
    );
    press_key(&mut app, KeyCode::Esc);
    let frame = redrawn(&mut app);
    assert!(frame.contains("· Single · Texas"), "put back: {frame}");
    picked(&mut app, 't', "where");
    app.world_mut().resource_mut::<YearCursor>().0 = Some(2060);
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("Tax Tables · 2060 · Single · Washington"),
        "{frame}"
    );
    assert!(composed_frame(&app).contains("] next year"));
}

#[test]
fn a_year_moved_while_away_is_shown_on_return() {
    let mut app = moving_app();
    assert!(redrawn(&mut app).contains("Tax Tables · 2026 ·"));
    show(&mut app, Page::Overview);
    press_key(&mut app, KeyCode::Right);
    show(&mut app, Page::TaxTables);
    let frame = redrawn(&mut app);
    assert!(frame.contains("Tax Tables · 2027 ·"), "{frame}");
}
