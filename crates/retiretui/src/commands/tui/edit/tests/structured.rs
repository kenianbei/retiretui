//! Headless tests for the fields that edit a table or a list the item holds.

use bevy_app::App;
use plurimus::term::KeyCode;
use retiretui_engine::plan::{Medicare, TreatmentClass};

use super::{draft_plan, fixture_app, focused, open, shows_row, tab_to_key};
use crate::commands::tui::edit::build::FormButton;
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{
    commit_edit, composed_frame, press_key, press_shift, said, type_text,
};

const MEDICARE: &str = "medicare";
const PART_D: &str = "medicare.part_d";
const PRIOR_MAGI: &str = "medicare.prior_magi";

fn tick(app: &mut App, key: &str) {
    tab_to_key(app, key);
    press_key(app, KeyCode::Char(' '));
    app.update();
}

fn medicare(app: &App) -> Option<Medicare> {
    draft_plan(app).medicare
}

#[test]
fn the_medicare_tick_makes_the_table_and_shows_its_rows_and_unticked_removes_both() {
    let mut app = fixture_app();
    open(&mut app, Page::Household);
    assert!(!composed_frame(&app).contains("Include Part D"));
    tick(&mut app, MEDICARE);
    let frame = composed_frame(&app);
    assert!(frame.contains("Include Part D") && frame.contains("Income year before"));
    press_key(&mut app, KeyCode::Enter);
    let ticked = Medicare {
        prior_magi: Vec::new(),
        part_d: true,
    };
    assert_eq!(medicare(&app), Some(ticked));
    // Applying closed the form; open it again.
    press_key(&mut app, KeyCode::Enter);
    tick(&mut app, MEDICARE);
    assert!(!shows_row(&composed_frame(&app), "Include Part D"));
    press_key(&mut app, KeyCode::Tab);
    let stop = app.world().get::<FormButton>(focused(&app));
    assert!(stop.is_some(), "a hidden row is no stop on the way round");
    press_shift(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(medicare(&app), None);
}

#[test]
fn an_earlier_income_alone_is_refused_and_both_are_written_oldest_first() {
    let mut app = fixture_app();
    open(&mut app, Page::Household);
    tick(&mut app, MEDICARE);
    tab_to_key(&mut app, PRIOR_MAGI);
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "180000");
    press_key(&mut app, KeyCode::Enter);
    let heard = said(&app).join("\n");
    assert!(heard.contains("Income last year: is blank"), "{heard}");
    assert_eq!(medicare(&app), None, "nothing was applied");
    press_shift(&mut app, KeyCode::Tab);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("$180,000"),
        "left, it reads as money: {frame}"
    );
    type_text(&mut app, "185000");
    press_key(&mut app, KeyCode::Enter);
    let held = medicare(&app).map(|held| held.prior_magi);
    assert_eq!(held, Some(vec![180_000, 185_000]));
}

#[test]
fn a_household_edited_beside_its_medicare_keeps_it_as_it_was() {
    let mut app = fixture_app();
    let stated = Medicare {
        prior_magi: vec![180_000, 185_000],
        part_d: true,
    };
    let wanted = stated.clone();
    commit_edit(&mut app, move |plan| plan.medicare = Some(stated.clone()));
    open(&mut app, Page::Household);
    let frame = composed_frame(&app);
    let last_year = frame.find("$185,000").expect("last year is shown");
    assert!(
        frame
            .find("$180,000")
            .is_some_and(|before| before > last_year)
    );
    tick(&mut app, PART_D);
    press_key(&mut app, KeyCode::Enter);
    let unticked = Medicare {
        part_d: false,
        ..wanted
    };
    assert_eq!(medicare(&app), Some(unticked), "the incomes are kept");
}

#[test]
fn a_household_without_medicare_gains_none_by_being_edited() {
    let mut app = fixture_app();
    let filing = draft_plan(&app).household.filing;
    open(&mut app, Page::Household);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Enter);
    assert_ne!(draft_plan(&app).household.filing, filing);
    assert_eq!(medicare(&app), None);
}

#[test]
fn unticking_medicare_lets_go_of_what_its_rows_left_unfinished() {
    let mut app = fixture_app();
    open(&mut app, Page::Household);
    tick(&mut app, MEDICARE);
    tab_to_key(&mut app, PRIOR_MAGI);
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "180000");
    press_shift(&mut app, KeyCode::Tab);
    press_shift(&mut app, KeyCode::Tab);
    press_shift(&mut app, KeyCode::Tab);
    tick(&mut app, MEDICARE);
    press_key(&mut app, KeyCode::Enter);
    assert!(
        !said(&app).join("\n").contains("is blank"),
        "nothing is refused"
    );
    assert_eq!(medicare(&app), None);
}

const ORDER: &str = "withdrawal_order";

fn open_settings(app: &mut App) {
    open(app, Page::Settings);
    tab_to_key(app, ORDER);
}

fn order(app: &App) -> Vec<TreatmentClass> {
    draft_plan(app).plan.withdrawal_order
}

#[test]
fn a_place_offers_only_what_no_other_holds_and_a_skipped_one_closes_up() {
    use TreatmentClass::{Deferred, Hsa, Roth, Taxable};

    let mut app = fixture_app();
    assert_eq!(order(&app), [Taxable, Deferred, Roth, Hsa]);
    open_settings(&mut app);
    press_key(&mut app, KeyCode::Right);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("[ Skip ▾ ]"),
        "every other kind is held: {frame}"
    );
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Left);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(
        order(&app),
        [Taxable, Roth, Hsa],
        "the freed kind is offered"
    );
}

#[test]
fn an_order_of_nothing_is_refused_rather_than_read_as_the_usual_one() {
    let mut app = fixture_app();
    let before = order(&app);
    open_settings(&mut app);
    for _ in 0..before.len() {
        press_key(&mut app, KeyCode::Right);
        press_key(&mut app, KeyCode::Tab);
    }
    press_shift(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Enter);
    let heard = said(&app).join("\n");
    assert!(
        heard.contains("Withdraw first: needs at least one"),
        "{heard}"
    );
    assert_eq!(order(&app), before);
}

#[test]
fn a_short_order_fills_the_first_places_and_survives_an_edit_beside_it() {
    use TreatmentClass::{Roth, Taxable};

    let mut app = fixture_app();
    commit_edit(&mut app, |plan| {
        plan.plan.withdrawal_order = vec![Roth, Taxable];
    });
    let before = draft_plan(&app).plan.inflation;
    open_settings(&mut app);
    let frame = composed_frame(&app);
    let (roth, taxable) = (frame.find("[ Roth ▾ ]"), frame.find("[ Taxable ▾ ]"));
    assert!(roth.is_some() && roth < taxable, "{frame}");
    assert_eq!(frame.matches("[ Skip ▾ ]").count(), 2, "{frame}");
    // Back over wage growth's input and slider to inflation's slider.
    for _ in 0..4 {
        press_shift(&mut app, KeyCode::Tab);
    }
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Enter);
    assert!(draft_plan(&app).plan.inflation > before, "the slider moved");
    assert_eq!(order(&app), [Roth, Taxable]);
}
