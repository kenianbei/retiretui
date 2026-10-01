//! Workplace-plan coverage stated on a salary, and the band a traditional
//! IRA contribution phases out over for the spouse of a covered person.

mod common;

use retiretui_engine::params::{Inflation, PhaseOut, TaxTables};
use retiretui_engine::project::{ContributionNote, Projection, project};

use common::{assert_issue, contribution, head, issues, plan_from, run, with_override};

const IRA: &str = r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 0

[[contributions]]
id = "to-ira"
to = "ira"
amount = 3000
cola = false
"#;

/// Dana beside `me`, and a traditional IRA each paid 3,000.
const JOINT: &str = r#"
[[household.people]]
id = "dana"
birth = 1981-03-01

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "ira-me"
kind = "ira"
owner = "me"
balance = 0

[[accounts]]
id = "ira-dana"
kind = "ira"
owner = "dana"
balance = 0

[[contributions]]
id = "to-ira-me"
to = "ira-me"
amount = 3000
cola = false

[[contributions]]
id = "to-ira-dana"
to = "ira-dana"
amount = 3000
cola = false
"#;

/// A joint household whose salaries set the MAGI: Dana's job covers Dana,
/// and the other covers no one.
fn joint(me: i64, dana: i64) -> String {
    let text = head(&format!(
        r#"{JOINT}
[[income]]
id = "salary-me"
kind = "salary"
owner = "me"
amount = {me}
cola = false

[[income]]
id = "salary-dana"
kind = "salary"
owner = "dana"
amount = {dana}
cola = false
covered = true
"#
    ));
    text.replace("filing = \"single\"", "filing = \"married-joint\"")
}

fn not_deducted(projection: &Projection, year: usize, account: &str) -> i64 {
    contribution(projection, year, account)
        .2
        .iter()
        .find_map(|note| match note {
            ContributionNote::NotDeducted { amount } => Some(*amount),
            _ => None,
        })
        .unwrap_or(0)
}

#[test]
fn a_covered_salary_phases_an_ira_out_while_it_pays() {
    let text = head(&format!(
        r#"{IRA}
[[income]]
id = "pay"
kind = "salary"
owner = "me"
amount = 100000
cola = false
covered = true
end = {{ date = 2026-12-31 }}

[[income]]
id = "consulting"
kind = "salary"
owner = "me"
amount = 100000
cola = false
start = {{ date = 2027-01-01 }}
"#
    ));
    let projection = run(&text);
    assert_eq!(
        not_deducted(&projection, 0, "ira"),
        3_000,
        "MAGI 100,000 is past the covered band's top, with nothing paid into a workplace plan"
    );
    assert_eq!(
        not_deducted(&projection, 1, "ira"),
        0,
        "the same MAGI deducts in full once the covered job has ended"
    );
}

#[test]
fn the_spouse_of_a_covered_person_phases_out_over_their_own_band() {
    let below = run(&joint(120_000, 120_000));
    assert_eq!(
        not_deducted(&below, 0, "ira-me"),
        0,
        "MAGI 240,000 is below the spouse band"
    );
    assert_eq!(
        not_deducted(&below, 0, "ira-dana"),
        3_000,
        "and past the covered band"
    );
    let middle = run(&joint(127_000, 120_000));
    assert_eq!(
        not_deducted(&middle, 0, "ira-me"),
        1_500,
        "247,000 is halfway"
    );
    assert_eq!(not_deducted(&middle, 0, "ira-dana"), 3_000);
    let top = run(&joint(140_000, 120_000));
    assert_eq!(not_deducted(&top, 0, "ira-me"), 3_000);
}

#[test]
fn a_year_whose_table_leaves_the_spouse_band_out_deducts_the_spouse_in_full() {
    let text = joint(127_000, 120_000).replace("start_year = 2026", "start_year = 2027");
    let projection = project(&plan_from(&text), &with_override());
    assert_eq!(not_deducted(&projection, 0, "ira-me"), 0);
    assert_eq!(not_deducted(&projection, 0, "ira-dana"), 3_000);
}

#[test]
fn the_spouse_band_is_inflated_and_an_override_year_may_leave_it_out() {
    let inflation = Inflation::constant(0.02);
    let embedded = TaxTables::embedded();
    let band = |tables: &TaxTables, year| {
        tables
            .params_for(year, &inflation)
            .limits
            .ira_deduction_phase_out_spouse
    };
    assert_eq!(
        band(&embedded, 2026),
        Some(PhaseOut {
            from: 242_000,
            to: 252_000
        })
    );
    let later = band(&embedded, 2030).unwrap();
    assert!(later.from > 242_000 && later.to > later.from, "{later:?}");
    assert_eq!(band(&with_override(), 2027), None);
}

#[test]
fn only_a_salary_makes_its_owner_covered() {
    let text = head(&format!(
        r#"{IRA}
[[income]]
id = "pension"
kind = "pension"
owner = "me"
amount = 1000
covered = true
"#
    ));
    assert_issue(
        &issues(&text),
        "income[0].covered",
        "only a salary makes its owner covered",
    );
}
