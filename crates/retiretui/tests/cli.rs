//! CLI behavior tests, driving the built binary.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{FULL_PLAN, OPT_PLAN, STATEMENT, json_of, retiretui, scratch_dir};

#[test]
fn validate_accepts_the_fixture_and_rejects_garbage() {
    let ok = retiretui(&["validate", FULL_PLAN]);
    assert!(ok.status.success(), "{ok:?}");
    let missing = retiretui(&["validate", "no-such-plan.toml"]);
    assert!(!missing.status.success());
    let scratch = std::env::temp_dir().join("retiretui-cli-bad-plan.toml");
    std::fs::write(&scratch, "schema = 1\n").unwrap();
    let bad = retiretui(&["validate", scratch.to_str().unwrap()]);
    assert!(!bad.status.success());
}

#[test]
fn project_renders_the_aggregate_table() {
    let output = retiretui(&["project", FULL_PLAN]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let header = stdout.lines().next().unwrap();
    for column in ["year", "age", "income", "expense", "tax", "deferred", "net"] {
        assert!(header.contains(column), "missing `{column}` in {header}");
    }
    assert!(stdout.contains("2026"));
}

#[test]
fn project_by_account_names_account_columns() {
    let output = retiretui(&["project", FULL_PLAN, "--by-account"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let header = stdout.lines().next().unwrap();
    assert!(header.contains("pension-dc"));
    assert!(header.contains("fid-401k"));
}

#[test]
fn project_nominal_differs_from_todays_dollars() {
    let real = retiretui(&["project", FULL_PLAN]);
    let nominal = retiretui(&["project", FULL_PLAN, "--nominal"]);
    let real = String::from_utf8(real.stdout).unwrap();
    let nominal = String::from_utf8(nominal.stdout).unwrap();
    // Year one has deflator 1.0, so the first data row matches (modulo
    // column padding); later years diverge.
    let tokens = |text: &str, line: usize| -> Vec<String> {
        text.lines()
            .nth(line)
            .unwrap()
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(tokens(&real, 1), tokens(&nominal, 1));
    assert_ne!(tokens(&real, 5), tokens(&nominal, 5));
}

#[test]
fn project_json_carries_nominal_amounts_and_deflators() {
    let parsed = json_of(&retiretui(&["project", FULL_PLAN, "--format", "json"]));
    let years = parsed["years"].as_array().unwrap();
    assert!(!years.is_empty());
    assert!((years[0]["deflator"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    assert!(years[0]["balances"].get("pension-dc").is_some());
}

const RETIRE_EARLY: &str = r#"
schema = 1
base = "base.toml"

[plan]
name = "retire-early"

[[expenses]]
id = "travel"
remove = true
"#;

#[test]
fn scenarios_resolve_in_validate_and_project() {
    let dir = scratch_dir("cli-scenario", "base.toml", &[("early.toml", RETIRE_EARLY)]);
    let early = dir.join("early.toml");
    let ok = retiretui(&["validate", early.to_str().unwrap()]);
    assert!(ok.status.success(), "{ok:?}");
    let output = retiretui(&["project", early.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn scenario_base_cycles_are_detected() {
    let a = "schema = 1\nbase = \"b.toml\"\n";
    let b = "schema = 1\nbase = \"a.toml\"\n";
    let dir = scratch_dir("cli-cycle", "base.toml", &[("a.toml", a), ("b.toml", b)]);
    let output = retiretui(&["validate", dir.join("a.toml").to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("cycle"), "{stderr}");
}

#[test]
fn compare_renders_summary_and_metric_tables() {
    let dir = scratch_dir("cli-compare", "base.toml", &[("early.toml", RETIRE_EARLY)]);
    let base = dir.join("base.toml");
    let early = dir.join("early.toml");
    let paths = [base.to_str().unwrap(), early.to_str().unwrap()];
    let output = retiretui(&["compare", paths[0], paths[1]]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let header = stdout.lines().next().unwrap();
    for column in ["plan", "final net", "peak", "taxes", "first unf"] {
        assert!(header.contains(column), "missing `{column}` in {header}");
    }
    assert!(stdout.contains("base"), "{stdout}");
    assert!(stdout.contains("retire-early"), "{stdout}");
    assert_eq!(stdout.lines().count(), 3, "one row per plan\n{stdout}");
    let metric = retiretui(&["compare", paths[0], paths[1], "--metric", "net-worth"]);
    assert!(metric.status.success(), "{metric:?}");
    let stdout = String::from_utf8(metric.stdout).unwrap();
    let header = stdout.lines().next().unwrap();
    for column in ["year", "base", "retire-early"] {
        assert!(header.contains(column), "missing `{column}` in {header}");
    }
    assert!(stdout.contains("2026"));
    let single = retiretui(&["compare", paths[0]]);
    assert!(!single.status.success(), "compare needs two plans");
}

#[test]
fn compare_metric_magi_reads_each_years_magi() {
    let dir = scratch_dir(
        "cli-compare-magi",
        "base.toml",
        &[("early.toml", RETIRE_EARLY)],
    );
    let base = dir.join("base.toml");
    let early = dir.join("early.toml");
    let paths = [base.to_str().unwrap(), early.to_str().unwrap()];
    let json = json_of(&retiretui(&[
        "compare", paths[0], paths[1], "--format", "json",
    ]));
    let magi = json["plans"][0]["years"][0]["taxes"]["magi"]
        .as_i64()
        .unwrap();
    let output = retiretui(&[
        "compare",
        paths[0],
        paths[1],
        "--metric",
        "magi",
        "--nominal",
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let first_year = stdout.lines().nth(1).unwrap();
    let cells: Vec<&str> = first_year.split_whitespace().collect();
    assert_eq!(cells[1], magi.to_string(), "{stdout}");
}

#[test]
fn compare_json_carries_summaries_and_years() {
    let dir = scratch_dir(
        "cli-compare-json",
        "base.toml",
        &[("early.toml", RETIRE_EARLY)],
    );
    let base = dir.join("base.toml");
    let early = dir.join("early.toml");
    let output = retiretui(&[
        "compare",
        base.to_str().unwrap(),
        early.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let parsed = json_of(&output);
    let plans = parsed["plans"].as_array().unwrap();
    assert_eq!(plans.len(), 2);
    assert_eq!(plans[1]["name"], "retire-early");
    assert!(plans[0]["summary"]["final_net_worth"].is_i64());
    assert!(!plans[0]["years"].as_array().unwrap().is_empty());
}

#[test]
fn tui_refuses_bad_plans_before_touching_the_terminal() {
    let missing = retiretui(&["tui", "no-such-plan.toml"]);
    assert!(!missing.status.success());
    let scratch = std::env::temp_dir().join("retiretui-cli-tui-bad.toml");
    std::fs::write(&scratch, "schema = 1\n").unwrap();
    let bad = retiretui(&["tui", scratch.to_str().unwrap()]);
    assert!(!bad.status.success());
    assert!(bad.stdout.is_empty(), "no TUI output on a refused plan");
    let refusal = String::from_utf8_lossy(&bad.stderr);
    assert!(!refusal.contains("panicked"), "{refusal}");
    assert!(refusal.contains("missing field"), "{refusal}");
}

#[test]
fn project_honors_tax_dir_overrides() {
    let dir = Path::new("../retiretui_engine/tests/fixtures/tax-override");
    let output = retiretui(&["project", FULL_PLAN, "--tax-dir", dir.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
}

/// 100,000 moved from the 401(k) to a brokerage, at 46.
const EARLY_TRANSFER: &str = r#"
[[accounts]]
id = "brokerage"
kind = "brokerage"
owner = "me"
balance = 0

[[transfers]]
id = "cash-out"
from = "k"
to = "brokerage"
amount = 100000
on = { date = 2026-01-01 }
"#;

#[test]
fn actions_says_the_penalty_an_early_transfer_pays() {
    let early = format!("{OPT_PLAN}{EARLY_TRANSFER}");
    let dir = scratch_dir("cli-penalty", "base.toml", &[("early.toml", &early)]);
    let plan = dir.join("early.toml");
    let plan = plan.to_str().unwrap();
    let paid = "Early-withdrawal penalty paid this year: $10,000";
    let said = retiretui(&["actions", plan, "--year", "2026"]);
    assert!(said.status.success(), "{said:?}");
    let text = String::from_utf8_lossy(&said.stdout).into_owned();
    assert!(text.contains(paid), "{text}");
    let json = retiretui(&["actions", plan, "--year", "2026", "--format", "json"]);
    assert_eq!(json_of(&json)["warnings"], serde_json::json!([paid]));
}

#[test]
fn actions_reports_one_year_and_rejects_out_of_range() {
    let year_2040 = retiretui(&["actions", FULL_PLAN, "--year", "2040"]);
    assert!(year_2040.status.success(), "{year_2040:?}");
    let text = String::from_utf8_lossy(&year_2040.stdout).into_owned();
    assert!(text.starts_with("Actions for 2040 (ages "), "{text}");
    assert!(text.contains(" $"), "amounts read as money: {text}");

    let json = retiretui(&["actions", FULL_PLAN, "--year", "2040", "--format", "json"]);
    let report = json_of(&json);
    assert_eq!(report["year"], 2040);
    assert!(report["actions"].is_array());

    let outside = retiretui(&["actions", FULL_PLAN, "--year", "1990"]);
    assert!(!outside.status.success());
    let error = String::from_utf8_lossy(&outside.stderr).into_owned();
    assert!(error.contains("the plan covers 2026-2074"), "{error}");
}

#[test]
fn import_earnings_records_the_statement_on_the_person() {
    let dir = scratch_dir(
        "cli-import-earnings",
        "base.toml",
        &[("early.toml", RETIRE_EARLY)],
    );
    let scenario = dir.join("early.toml");
    let refused = retiretui(&[
        "import-earnings",
        scenario.to_str().unwrap(),
        STATEMENT,
        "--person",
        "jordan",
    ]);
    assert!(!refused.status.success());
    let stderr = String::from_utf8(refused.stderr).unwrap();
    assert!(stderr.contains("scenario"), "{stderr}");
    let plan = dir.join("base.toml");
    let plan = plan.to_str().unwrap();
    let wrong = retiretui(&["import-earnings", plan, STATEMENT, "--person", "alex"]);
    assert!(!wrong.status.success());
    let stderr = String::from_utf8(wrong.stderr).unwrap();
    assert!(stderr.contains("born"), "{stderr}");
    let ok = retiretui(&["import-earnings", plan, STATEMENT, "--person", "jordan"]);
    assert!(ok.status.success(), "{ok:?}");
    let stdout = String::from_utf8(ok.stdout).unwrap();
    assert!(
        stdout.contains("3 years of earnings (1995-2024)"),
        "{stdout}"
    );
    let written = std::fs::read_to_string(plan).unwrap();
    assert!(written.contains("[household.people.earnings]"), "{written}");
    assert!(written.contains("2024 = 168600"), "{written}");
    let valid = retiretui(&["validate", plan]);
    assert!(valid.status.success(), "{valid:?}");
}

const DUSK: &str = "# mine\nfamily = \"mine\"\nvariant = \"dark\"\naccent = \"#010203\"\n";
const LIGHT_GROUND: &str = "0;15";
const DARK_GROUND: &str = "15;0";
const EMBEDDED_THEMES: usize = 13;

/// A config home of this test's own, its `themes` directory holding
/// `themes`.
fn config_home(name: &str, themes: &[(&str, &str)]) -> PathBuf {
    let home = std::env::temp_dir().join(format!("retiretui-theme-{name}"));
    let _ = std::fs::remove_dir_all(&home);
    let directory = home.join("retiretui/themes");
    std::fs::create_dir_all(&directory).unwrap();
    for (file, text) in themes {
        std::fs::write(directory.join(file), text).unwrap();
    }
    home
}

/// `retiretui theme` under the config home `home`, on a terminal whose
/// `COLORFGBG` is `ground`.
fn theme(home: &Path, ground: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_retiretui"))
        .arg("theme")
        .args(args)
        .env("XDG_CONFIG_HOME", home)
        .env("COLORFGBG", ground)
        .output()
        .unwrap()
}

fn text_of(output: &[u8]) -> String {
    String::from_utf8(output.to_vec()).unwrap()
}

fn embedded_theme(slug: &str) -> String {
    let themes = Path::new("../retiretui_tui/src/theme/themes");
    std::fs::read_to_string(themes.join(format!("{slug}.toml"))).unwrap()
}

#[test]
fn theme_list_names_the_built_in_themes_and_the_users_beside_them() {
    let files = [
        ("dusk.toml", DUSK),
        ("nord.toml", DUSK),
        ("bad.toml", "family = 3"),
        ("terminal.toml", DUSK),
        ("notes.md", DUSK),
    ];
    let home = config_home("list", &files);
    let listed = theme(&home, DARK_GROUND, &["list"]);
    assert!(listed.status.success(), "{listed:?}");
    let stdout = text_of(&listed.stdout);
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(rows[0], ["theme", "variant", "family", "from"]);
    assert_eq!(rows[1][0], "terminal");
    let row = |slug: &str| rows.iter().find(|row| row[0] == slug).unwrap()[1..].join(" ");
    assert_eq!(row("dusk"), "dark mine user");
    assert_eq!(row("nord"), "dark mine user, over built in");
    assert_eq!(row("bad"), "does not read");
    assert_eq!(row("gruvbox-light"), "light gruvbox built in");
    let built_in = stdout.lines().filter(|line| line.ends_with("  built in"));
    assert_eq!(built_in.count(), EMBEDDED_THEMES - 1);
    assert_eq!(rows.len(), EMBEDDED_THEMES + 4, "{stdout}");
    let stderr = text_of(&listed.stderr);
    assert!(stderr.contains("terminal.toml: terminal is"), "{stderr}");
    assert!(stderr.contains("theme bad: "), "{stderr}");
    assert_eq!(stderr.lines().count(), 2, "{stderr}");
}

#[test]
fn theme_list_with_no_themes_of_the_users_is_the_built_in_ones_and_says_nothing() {
    let home = std::env::temp_dir().join("retiretui-theme-none");
    let _ = std::fs::remove_dir_all(&home);
    let listed = theme(&home, DARK_GROUND, &["list"]);
    assert!(listed.status.success(), "{listed:?}");
    assert_eq!(text_of(&listed.stdout).lines().count(), EMBEDDED_THEMES + 2);
    assert_eq!(text_of(&listed.stderr), "");
}

#[test]
fn theme_dump_prints_the_file_a_session_would_take_the_name_as() {
    let home = config_home("dump", &[("dusk.toml", DUSK), ("nord.toml", DUSK)]);
    let dumped = |ground, name| {
        let output = theme(&home, ground, &["dump", name]);
        assert!(output.status.success(), "{output:?}");
        text_of(&output.stdout)
    };
    assert_eq!(dumped(DARK_GROUND, "dracula"), embedded_theme("dracula"));
    let light = embedded_theme("gruvbox-light");
    assert_eq!(dumped(LIGHT_GROUND, "gruvbox"), light, "a family");
    assert_eq!(
        dumped(DARK_GROUND, "gruvbox"),
        embedded_theme("gruvbox-dark")
    );
    assert_eq!(dumped(DARK_GROUND, "dusk"), DUSK, "the user's, as written");
    assert_eq!(
        dumped(DARK_GROUND, "nord"),
        DUSK,
        "theirs over the built in"
    );
    assert_eq!(dumped(LIGHT_GROUND, "mine"), DUSK);

    let copy = home.join("retiretui/themes/copy.toml");
    std::fs::write(&copy, dumped(DARK_GROUND, "tokyo-night")).unwrap();
    let listed = text_of(&theme(&home, DARK_GROUND, &["list"]).stdout);
    let copied = listed.lines().find(|line| line.starts_with("copy "));
    assert!(
        copied.is_some_and(|row| row.ends_with("  user")),
        "{listed}"
    );
}

#[test]
fn theme_dump_refuses_the_terminals_own_and_a_name_that_is_no_theme() {
    let home = config_home("refused", &[]);
    for (name, said) in [("terminal", "have no file"), ("nrod", "\"nrod\"")] {
        let refused = theme(&home, DARK_GROUND, &["dump", name]);
        assert!(!refused.status.success(), "{name}");
        assert_eq!(text_of(&refused.stdout), "");
        let stderr = text_of(&refused.stderr);
        assert!(stderr.contains(said), "{stderr}");
    }
}
