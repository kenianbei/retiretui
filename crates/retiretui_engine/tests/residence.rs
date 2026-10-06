//! Where the household lives, year by year, and what its state takes.

use retiretui_engine::params::{Inflation, TaxTables};
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{Projection, project, state_lived_in, validate_plan};

const PLAN: &str = r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 50
inflation = 0.0

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1980-06-15

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "income-1"
kind = "salary"
owner = "me"
amount = 100000
"#;

fn plan(residency: &str) -> Plan {
    Plan::from_toml_str(&format!("{PLAN}\n{residency}")).unwrap()
}

fn run(residency: &str) -> Projection {
    let plan = plan(residency);
    let issues = validate_plan(&plan, &TaxTables::embedded());
    assert!(issues.is_empty(), "invalid test plan: {issues:?}");
    project(&plan, &TaxTables::embedded())
}

fn state_taxes(projection: &Projection) -> Vec<i64> {
    let rows = projection.years.iter();
    rows.map(|row| row.taxes.state).collect()
}

const OREGON: &str = "[[residency]]\ncountry = \"us\"\nstate = \"or\"\n";

#[test]
fn a_state_takes_its_tax_on_top_of_an_unchanged_federal_one() {
    let nowhere = run("");
    let oregon = run(OREGON);
    assert!(state_taxes(&nowhere).iter().all(|&tax| tax == 0));
    for (bare, taxed) in nowhere.years.iter().zip(&oregon.years) {
        // 13,170 of federal tax, 8,750 of it subtracted with the 2,910:
        // 216 + 462 + 8.75% of 76,940, less the credit of 263.
        assert_eq!(taxed.taxes.state, 7_148);
        assert_eq!(taxed.taxes.ordinary, bare.taxes.ordinary);
        assert_eq!(taxed.taxes.total, bare.taxes.total + 7_148);
        assert_eq!(taxed.surplus, bare.surplus - 7_148);
    }
}

#[test]
fn a_move_is_taxed_by_the_new_state_for_its_whole_year() {
    let moved = format!(
        "{OREGON}\n[[residency]]\ncountry = \"us\"\nstate = \"tx\"\nfrom = {{ date = 2028-09-01 }}\n"
    );
    assert_eq!(state_taxes(&run(&moved)), [7_148, 7_148, 0, 0, 0]);
    let abroad =
        format!("{OREGON}\n[[residency]]\ncountry = \"pt\"\nfrom = {{ date = 2029-01-01 }}\n");
    assert_eq!(state_taxes(&run(&abroad)), [7_148, 7_148, 7_148, 0, 0]);
}

#[test]
fn of_two_moves_in_a_year_the_one_listed_later_stands_whatever_the_order_of_the_rest() {
    let there_and_back = "[[residency]]\ncountry = \"us\"\nstate = \"tx\"\nfrom = { date = 2029-03-01 }\n\n\
        [[residency]]\ncountry = \"us\"\nstate = \"fl\"\nfrom = { date = 2027-01-01 }\n\n\
        [[residency]]\ncountry = \"us\"\nstate = \"or\"\nfrom = { date = 2027-06-01 }\n\n\
        [[residency]]\ncountry = \"us\"\nstate = \"wa\"\n";
    assert_eq!(state_taxes(&run(there_and_back)), [0, 7_148, 7_148, 0, 0]);
}

#[test]
fn a_state_with_no_table_is_refused_rather_than_taxed_at_nothing() {
    let california = plan(
        "[[residency]]\ncountry = \"us\"\nstate = \"ca\"\nfrom = { date = 2030-01-01 }\n\n[[residency]]\ncountry = \"us\"\nstate = \"or\"\n",
    );
    assert!(california.validate().is_empty(), "a known place");
    let issues = validate_plan(&california, &TaxTables::embedded());
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].path, "residency[0].state");
    assert!(issues[0].message.contains("not modeled"), "{issues:?}");
}

#[test]
fn an_override_year_without_states_inherits_them_inflated() {
    let text = std::fs::read_to_string("tax/2026.toml").unwrap();
    let (federal, _) = text.split_once("[states.").unwrap();
    let mut tables = TaxTables::embedded();
    tables
        .add_source(
            &federal.replace("year = 2026", "year = 2028"),
            "a later year",
        )
        .unwrap();
    assert_eq!(validate_plan(&plan(OREGON), &tables), []);
    let inflation = Inflation::constant(0.025);
    let carried = TaxTables::embedded().params_for(2030, &inflation);
    let inherited = tables.params_for(2030, &inflation);
    assert_eq!(inherited.states["or"], carried.states["or"]);
    assert!(!inherited.states.contains_key("ca"));
    let projected = state_taxes(&project(&plan(OREGON), &tables));
    assert_eq!(projected, state_taxes(&run(OREGON)));
    assert!(projected.iter().all(|&tax| tax > 0));
}

#[test]
fn a_later_year_s_table_that_restates_a_state_stands_from_that_year() {
    let text = std::fs::read_to_string("tax/2026.toml").unwrap();
    let restated = text
        .replace("year = 2026", "year = 2028")
        .replace("single = 2910", "single = 3000");
    let mut tables = TaxTables::embedded();
    tables.add_source(&restated, "a later year").unwrap();
    let deduction = |year| {
        let params = tables.params_for(year, &Inflation::constant(0.0));
        params.states["or"].deduction.single
    };
    assert_eq!(
        [deduction(2027), deduction(2028), deduction(2030)],
        [2_910, 3_000, 3_000]
    );
}

#[test]
fn the_state_lived_in_follows_a_move_and_is_none_abroad() {
    let moving = plan(&format!(
        "{OREGON}\n[[residency]]\ncountry = \"us\"\nstate = \"tx\"\nfrom = {{ date = 2028-09-01 }}\n\n\
         [[residency]]\ncountry = \"pt\"\nfrom = {{ date = 2030-01-01 }}\n"
    ));
    assert_eq!(state_lived_in(&moving, 2027), Some("or"));
    assert_eq!(state_lived_in(&moving, 2028), Some("tx"));
    assert_eq!(state_lived_in(&moving, 2030), None);
    assert_eq!(state_lived_in(&plan(""), 2027), None);
}
