use bevy_app::App;
use plurimus::term::KeyCode;

use super::guide::{self, Place};
use super::people_tests::{BENEFIT, SALARY, app_on, fixture, plan, run, without_record};
use super::*;
use crate::commands::tui::support::{composed_frame, press_ctrl, press_key, redrawn, type_text};

/// A salaried person with no record and no Social Security income: where
/// a new plan starts.
fn fresh() -> String {
    let plan = fixture()
        .replace(BENEFIT, "")
        .replace("[[expenses]]", &format!("{SALARY}\n[[expenses]]"));
    without_record(&plan)
}

/// The line under the page: the frame's last but the key row, from the
/// sidebar's border on.
fn help_line(frame: &str) -> String {
    let lines: Vec<&str> = frame.lines().collect();
    let line = lines[lines.len() - 2];
    line.rsplit_once('╯')
        .map_or(line, |(_, help)| help)
        .trim()
        .to_owned()
}

#[test]
fn enter_on_a_person_offers_what_fits_them_and_runs_it_on_them() {
    let mut app = app_on(&fixture());
    press_key(&mut app, KeyCode::Enter);
    let frame = redrawn(&mut app);
    for offered in [
        "Import statement…",
        "Estimate from salary",
        "Clear record…",
        "Remove Social Security…",
    ] {
        assert!(frame.contains(offered), "{offered}: {frame}");
    }
    assert!(
        !frame.contains("Compute from record"),
        "nothing is typed: {frame}"
    );

    type_text(&mut app, "clear");
    press_key(&mut app, KeyCode::Enter);
    let frame = redrawn(&mut app);
    assert!(frame.contains("Clear me's earnings record?"), "{frame}");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert!(plan(&app).person("me").unwrap().earnings.is_empty());
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert!(
        !plan(&app).person("me").unwrap().earnings.is_empty(),
        "one step"
    );
}

#[test]
fn a_removal_asks_and_a_cancel_keeps_the_income() {
    let mut app = app_on(&fixture());
    assert_eq!(run(&mut app, remove_benefit), Outcome::Done);
    assert!(redrawn(&mut app).contains("Remove me's Social Security income?"));
    press_key(&mut app, KeyCode::Esc);
    app.update();
    assert_eq!(plan(&app).income.len(), 1);
    assert_eq!(
        run(&mut app, clear_record),
        Outcome::Done,
        "a record to clear is asked about"
    );
    press_key(&mut app, KeyCode::Esc);
    let no_record = fresh();
    let mut app = app_on(&no_record);
    assert_eq!(
        run(&mut app, clear_record),
        Outcome::Refused("me has no earnings record".to_owned())
    );
    assert_eq!(
        run(&mut app, remove_benefit),
        Outcome::Refused("me has no Social Security income".to_owned())
    );
}

/// What the line says with the keyboard on `place`, for the People
/// cursor's person.
fn said_on(app: &App, place: Place) -> String {
    let draft = app.world().resource::<Draft>();
    let person = draft.plan.household.people.first();
    guide::help_line(draft, person, place)
}

#[test]
fn the_help_line_names_the_next_thing_to_do_and_for_whom() {
    let mut app = app_on(&fresh());
    let frame = composed_frame(&app);
    assert!(
        help_line(&frame).starts_with("me has no earnings record: ⏎ on me"),
        "{frame}"
    );

    assert_eq!(run(&mut app, fill_career), Outcome::Done);
    let line = said_on(&app, Place::People);
    assert!(line.starts_with("⇥ to the claim options"), "{line}");
    assert!(line.ends_with("e, c and k act on me."), "{line}");
    let line = said_on(&app, Place::Strategies);
    assert!(line.starts_with("⏎ takes the highlighted option"), "{line}");

    assert_eq!(run(&mut app, adopt), Outcome::Done);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let line = said_on(&app, Place::People);
    assert!(
        line.starts_with("⏎ on a person for what can be done"),
        "{line}"
    );
}
