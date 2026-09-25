//! `monte-carlo` and `historical` CLI behavior tests, driving the built
//! binary.

mod common;

use common::{FULL_PLAN, json_of, retiretui, scratch_dir};

#[test]
fn monte_carlo_reports_its_markets_and_takes_flags() {
    let drawn = json_of(&retiretui(&[
        "monte-carlo",
        FULL_PLAN,
        "--trials",
        "40",
        "--draw",
        "history",
        "--format",
        "json",
    ]));
    assert_eq!(drawn["runs"], 40);
    assert_eq!(drawn["draw"], "history");
    let markets = drawn["markets"].as_array().unwrap();
    let names: Vec<&str> = markets
        .iter()
        .map(|market| market["market"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "as planned",
            "90th percentile",
            "75th percentile",
            "50th percentile",
            "25th percentile",
            "10th percentile",
            "worst"
        ]
    );
    assert_eq!(drawn["bands"][0]["year"], 2026);
}

#[test]
fn monte_carlo_reads_its_settings_from_a_scenario_and_refuses_bad_flags() {
    let dir = scratch_dir(
        "cli-monte-carlo",
        "base.toml",
        &[(
            "fewer.toml",
            "schema = 1\nbase = \"base.toml\"\n\n[market.monte_carlo]\ntrials = 25\nseed = 7\n",
        )],
    );
    let scenario = dir.join("fewer.toml");
    let fewer = json_of(&retiretui(&[
        "monte-carlo",
        scenario.to_str().unwrap(),
        "--format",
        "json",
    ]));
    assert_eq!(
        (fewer["runs"].as_u64(), fewer["seed"].as_u64()),
        (Some(25), Some(7))
    );

    let text = retiretui(&["monte-carlo", FULL_PLAN, "--trials", "10"]);
    let stdout = String::from_utf8(text.stdout).unwrap();
    assert!(stdout.contains("of 10 markets"), "{stdout}");
    let refused = retiretui(&["monte-carlo", FULL_PLAN, "--trials", "0"]);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8(refused.stderr)
            .unwrap()
            .contains("market.monte_carlo.trials")
    );
}

#[test]
fn historical_runs_each_start_year_and_reads_a_record_in_place_of_its_own() {
    let decade = json_of(&retiretui(&[
        "historical",
        FULL_PLAN,
        "--from",
        "1900",
        "--to",
        "1909",
        "--format",
        "json",
    ]));
    assert_eq!(decade["runs"], 10);
    assert_eq!(decade["start_years"].as_array().unwrap().len(), 11);
    assert_eq!(decade["start_years"][0]["market"], "as planned");

    let unwrapped = retiretui(&["historical", FULL_PLAN, "--from", "2000", "--no-wrap"]);
    assert!(!unwrapped.status.success());
    assert!(
        String::from_utf8(unwrapped.stderr)
            .unwrap()
            .contains("market.historical.wrap")
    );

    let dir = scratch_dir(
        "cli-history-file",
        "base.toml",
        &[(
            "record.toml",
            "years = [\n  { year = 2000, stocks = 0.1, bonds = 0.0, cash = 0.0, inflation = 0.0 },\n  { year = 2001, stocks = 0.0, bonds = 0.1, cash = 0.0, inflation = 0.0 },\n]\n",
        )],
    );
    let record = dir.join("record.toml");
    let own = json_of(&retiretui(&[
        "historical",
        FULL_PLAN,
        "--history",
        record.to_str().unwrap(),
        "--from",
        "2000",
        "--to",
        "2001",
        "--format",
        "json",
    ]));
    assert_eq!(own["runs"], 2);
}
