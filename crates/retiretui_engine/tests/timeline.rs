//! When a plan's dated items land: each resolved year is the one the
//! projection itself acts in.

mod common;

use retiretui_engine::project::{Projection, Timeline, YearRow};

use common::{head, plan_from, run};

const ITEMS: &str = r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 100000
allocation = [
  { from = { date = 2026-01-01 }, stocks = 1.0 },
  { from = { age = 60, owner = "me" }, bonds = 1.0 },
]

[market.stocks]
mean = 0.10

[market.bonds]
mean = 0.0

[[events]]
id = "retire"
trigger = { age = 62, owner = "me" }

[[income]]
id = "salary"
kind = "salary"
owner = "me"
amount = 100000

[[income]]
id = "pension"
kind = "pension"
owner = "me"
amount = 20000
start = { event = "retire", offset = 2 }

[[income]]
id = "ss"
kind = "social-security"
owner = "me"
start = { age = 67, owner = "me" }

[[contributions]]
id = "save"
to = "k"
amount = 10000
end = { event = "retire" }

[[residency]]
country = "us"
state = "or"

[[residency]]
country = "us"
state = "tx"
from = { date = 2030-01-01 }
"#;

fn timeline() -> Timeline {
    Timeline::new(&plan_from(&head(ITEMS)))
}

fn first_year(projection: &Projection, is_so: impl Fn(&YearRow) -> bool) -> Option<i16> {
    projection
        .years
        .iter()
        .find(|row| is_so(row))
        .map(|row| row.year)
}

fn pays(row: &YearRow, income: &str) -> bool {
    row.income.get(income).is_some_and(|&paid| paid > 0)
}

#[test]
fn an_event_at_an_age_lands_in_the_year_the_age_is_reached() {
    let at_62 = first_year(&run(&head(ITEMS)), |row| row.ages["me"] == 62);
    assert_eq!(timeline().events["retire"], Some(2042));
    assert_eq!(at_62, Some(2042));
}

#[test]
fn an_income_on_an_event_starts_the_offset_after_it() {
    let window = timeline().income["pension"];
    assert_eq!(
        (window.start, window.end, window.on),
        (Some(2044), None, None)
    );
    assert_eq!(window.first(), Some(2044));
    assert_eq!(
        first_year(&run(&head(ITEMS)), |row| pays(row, "pension")),
        Some(2044)
    );
}

#[test]
fn a_social_security_claim_at_an_age_lands_in_the_first_year_it_pays() {
    assert_eq!(timeline().income["ss"].first(), Some(2047));
    assert_eq!(
        first_year(&run(&head(ITEMS)), |row| pays(row, "ss")),
        Some(2047)
    );
}

#[test]
fn a_contribution_ending_on_an_event_is_last_paid_that_year() {
    let window = timeline().contributions["save"];
    assert_eq!((window.start, window.end), (None, Some(2042)));
    let unpaid = first_year(&run(&head(ITEMS)), |row| row.contributions_employee == 0);
    assert_eq!(unpaid, Some(2043));
}

#[test]
fn a_residency_move_lands_in_the_first_year_taxed_there() {
    assert_eq!(timeline().residency, [None, Some(2030)]);
    let untaxed = first_year(&run(&head(ITEMS)), |row| row.taxes.state == 0);
    assert_eq!(untaxed, Some(2030));
}

#[test]
fn a_glide_step_at_an_age_lands_in_the_first_year_it_is_held() {
    assert_eq!(timeline().glide_steps["k"], [Some(2026), Some(2040)]);
    assert!(!timeline().glide_steps.contains_key("cash"));
    let flat = first_year(&run(&head(ITEMS)), |row| !row.growth.contains_key("k"));
    assert_eq!(flat, Some(2040));
}
