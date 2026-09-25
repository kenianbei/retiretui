//! End-to-end golden test: the full fixture plan's projection, compared
//! byte-for-byte against a committed JSON snapshot so any numeric drift in
//! the engine fails loudly.

mod common;

use std::path::Path;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::project;

use common::FULL;

const GOLDEN_PATH: &str = "tests/fixtures/full-projection.json";

#[test]
fn projection_matches_golden_fixture() {
    let plan = Plan::from_toml_str(FULL).unwrap();
    assert!(plan.validate().is_empty());
    let projection = project(&plan, &TaxTables::embedded());
    let actual = serde_json::to_string_pretty(&projection).unwrap() + "\n";
    if std::env::var_os("REGEN_GOLDEN").is_some() {
        std::fs::write(GOLDEN_PATH, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(Path::new(GOLDEN_PATH)).unwrap();
    assert_eq!(
        actual, expected,
        "projection drifted from the golden fixture; rerun with REGEN_GOLDEN=1 only if the change is intended"
    );
}
