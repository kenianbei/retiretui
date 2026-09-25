//! Fill-bracket optimizer tests over a zero-inflation plan, so bracket
//! tops and deductions stay constant and every figure hand-checks.

mod common;

use retiretui_engine::optimize::{
    OptimizeOptions, OptimizedLadder, SweptBracket, apply_ladder, ladder_overlay,
    optimize_conversions, sweep_brackets,
};
use retiretui_engine::params::{Inflation, TaxTables};
use retiretui_engine::plan::{Plan, Scenario};
use retiretui_engine::project::{Projection, project};

use common::plan_from;

const BASE: &str = r#"
schema = 1

[plan]
name = "opt-base"
start_year = 2026
horizon_age = 75
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
balance = 200000

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 300000

[[accounts]]
id = "r"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[income]]
id = "salary"
kind = "salary"
owner = "me"
amount = 50000
cola = false
end = { age = 60, owner = "me" }

[[expenses]]
id = "living"
amount = 40000
cola = false
"#;

fn options() -> OptimizeOptions {
    OptimizeOptions {
        sources: vec!["k".to_owned()],
        destination: "r".to_owned(),
        start_year: None,
        end_year: None,
        annual_max: None,
        total_max: None,
        headroom: 0,
        irmaa_tier: None,
        max_magi: None,
    }
}

fn searched(plan: &Plan, options: &OptimizeOptions, rate: f64) -> SweptBracket {
    optimize_conversions(plan, &TaxTables::embedded(), options, rate)
        .unwrap()
        .ladder
}

/// The 12% bracket's top for a single filer at zero inflation.
fn twelve_top() -> i64 {
    let params = TaxTables::embedded().params_for(2026, &Inflation::constant(0.0));
    let brackets = params
        .brackets
        .for_status(retiretui_engine::plan::FilingStatus::Single);
    let index = brackets
        .iter()
        .position(|bracket| (bracket.rate - 0.12).abs() < 1e-9)
        .unwrap();
    brackets[index + 1].over
}

fn taxable_in(projection: &Projection, year: i16) -> i64 {
    projection
        .years
        .iter()
        .find(|row| row.year == year)
        .unwrap()
        .taxes
        .ordinary_taxable
}

#[test]
fn fills_to_the_bracket_top() {
    let ladder =
        optimize_conversions(&plan_from(BASE), &TaxTables::embedded(), &options(), 0.12).unwrap();
    let top = twelve_top();
    assert!(!ladder.ladder.steps.is_empty());
    for year in 2026..=2030 {
        assert_eq!(taxable_in(&ladder.ladder.optimized, year), top, "{year}");
        assert!(taxable_in(&ladder.baseline, year) < top, "{year}");
    }
    // Default window ends the year before RMDs begin (age 75 in 2055 for a
    // 1980 birth; window is capped by the horizon in this plan).
    assert!(ladder.ladder.steps.iter().all(|step| step.year >= 2026));
    assert_eq!(
        ladder.ladder.converted(false),
        ladder
            .ladder
            .steps
            .iter()
            .map(|step| step.amount)
            .sum::<i64>()
    );
}

#[test]
fn headroom_and_annual_and_total_caps_bind() {
    let mut cushioned = options();
    cushioned.headroom = 1000;
    let ladder = searched(&plan_from(BASE), &cushioned, 0.12);
    assert_eq!(taxable_in(&ladder.optimized, 2026), twelve_top() - 1000);

    let mut capped = options();
    capped.annual_max = Some(5000);
    let ladder = searched(&plan_from(BASE), &capped, 0.12);
    for step in &ladder.steps {
        assert!(step.amount <= 5000, "{step:?}");
    }
    assert_eq!(ladder.steps[0].amount, 5000, "cap binds below the target");

    let mut limited = options();
    limited.total_max = Some(20000);
    let ladder = searched(&plan_from(BASE), &limited, 0.12);
    assert_eq!(ladder.converted(false), 20000);
}

#[test]
fn locked_sources_wait_for_their_unlock() {
    let locked = BASE.replace(
        "id = \"k\"\nkind = \"401k\"",
        "id = \"k\"\nkind = \"401k\"\nlocked_until = { date = 2030-01-01 }",
    );
    let plan = plan_from(&locked);
    let ladder = searched(&plan, &options(), 0.12);
    assert!(
        ladder.steps.iter().all(|step| step.year >= 2030),
        "{:?}",
        ladder.steps
    );
    assert!(ladder.steps.iter().any(|step| step.year == 2030));
}

#[test]
fn sources_drain_in_the_given_order() {
    let two_sources = BASE.replace(
        "[[income]]",
        "[[accounts]]\nid = \"k2\"\nkind = \"ira\"\nowner = \"me\"\nbalance = 300000\n\n[[income]]",
    );
    let small = two_sources.replace(
        "id = \"k\"\nkind = \"401k\"\nowner = \"me\"\nbalance = 300000",
        "id = \"k\"\nkind = \"401k\"\nowner = \"me\"\nbalance = 10000",
    );
    let plan = plan_from(&small);
    let mut ordered = options();
    ordered.sources = vec!["k".to_owned(), "k2".to_owned()];
    let ladder = searched(&plan, &ordered, 0.12);
    let first_year: Vec<_> = ladder
        .steps
        .iter()
        .filter(|step| step.year == 2026)
        .collect();
    assert_eq!(first_year.len(), 2, "{:?}", ladder.steps);
    assert_eq!(first_year[0].source, "k");
    assert_eq!(first_year[0].amount, 10000, "k exhausts first");
    assert_eq!(first_year[1].source, "k2");
    assert_eq!(
        taxable_in(&ladder.optimized, 2026),
        twelve_top(),
        "the pair still fills the bracket"
    );
}

#[test]
fn existing_conversions_layer_underneath() {
    let layered = BASE.replace(
        "[[expenses]]",
        "[[conversions]]\nid = \"conversion-1\"\nfrom = \"k\"\nto = \"r\"\namount = 5000\ncola = false\n\n[[expenses]]",
    );
    let plan = plan_from(&layered);
    let bare = searched(&plan_from(BASE), &options(), 0.12);
    let ladder = searched(&plan, &options(), 0.12);
    assert_eq!(
        ladder.steps[0].amount,
        bare.steps[0].amount - 5000,
        "the base ladder fills part of the bracket"
    );
    assert_eq!(taxable_in(&ladder.optimized, 2026), twelve_top());
}

#[test]
fn overlay_round_trips_into_the_optimized_projection() {
    let base_plan = plan_from(BASE);
    let ladder = searched(&base_plan, &options(), 0.12);
    let text = ladder_overlay("base.toml", &plan_from(BASE), &options(), &ladder.steps).unwrap();
    assert!(
        text.starts_with("schema = 1\nbase = \"base.toml\"\n"),
        "{text}"
    );
    assert!(
        text.contains("[conversions.on]\ndate = 2026-01-01\n"),
        "{text}"
    );
    let scenario = Scenario::from_toml_str(&text).unwrap().expect("a scenario");
    assert_eq!(scenario.base(), "base.toml");
    let merged = scenario.apply(toml::from_str(BASE).unwrap()).unwrap();
    let merged_plan = Plan::from_toml_table(merged).unwrap();
    assert!(
        merged_plan.validate().is_empty(),
        "{:?}",
        merged_plan.validate()
    );
    assert_eq!(
        project(&merged_plan, &TaxTables::embedded()),
        ladder.optimized,
        "the emitted scenario reproduces the optimized projection"
    );
}

/// The plan with the 12% ladder taken into it, as the TUI takes one.
fn with_ladder_taken() -> Plan {
    let taken = searched(&plan_from(BASE), &options(), 0.12);
    let mut plan = plan_from(BASE);
    apply_ladder(&mut plan, &options(), &taken.steps);
    plan
}

#[test]
fn a_new_ladder_replaces_the_one_taken() {
    let laddered = with_ladder_taken();
    let bare = searched(&plan_from(BASE), &options(), 0.22);
    let again = optimize_conversions(&laddered, &TaxTables::embedded(), &options(), 0.22).unwrap();
    assert_eq!(
        again.ladder.steps, bare.steps,
        "searched as if no ladder were taken"
    );
    assert_eq!(again.ladder.optimized, bare.optimized);
    assert_eq!(
        again.baseline,
        project(&laddered, &TaxTables::embedded()),
        "the plan as given"
    );
    let swept = sweep_brackets(&laddered, &TaxTables::embedded(), &options()).unwrap();
    let fresh = sweep_brackets(&plan_from(BASE), &TaxTables::embedded(), &options()).unwrap();
    let steps = |sweep: &retiretui_engine::optimize::BracketSweep| {
        sweep
            .brackets
            .iter()
            .map(|bracket| bracket.steps.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(steps(&swept), steps(&fresh));
}

#[test]
fn an_overlay_removes_the_ladder_years_it_does_not_restate() {
    let laddered = with_ladder_taken();
    let mut shorter = options();
    shorter.end_year = Some(2027);
    let ladder = searched(&laddered, &shorter, 0.12);
    let text = ladder_overlay("base.toml", &laddered, &shorter, &ladder.steps).unwrap();
    assert!(text.contains("remove = true"), "{text}");
    let scenario = Scenario::from_toml_str(&text).unwrap().expect("a scenario");
    let base = toml::Table::try_from(&laddered).unwrap();
    let merged = Plan::from_toml_table(scenario.apply(base).unwrap()).unwrap();
    assert_eq!(
        merged.conversions.len(),
        ladder.steps.len(),
        "one ladder, the new one"
    );
    assert_eq!(project(&merged, &TaxTables::embedded()), ladder.optimized);
}

#[test]
fn converted_follows_the_basis() {
    let inflating = BASE.replace("inflation = 0.0", "inflation = 0.025");
    let plan = plan_from(&inflating);
    let ladder = searched(&plan, &options(), 0.12);
    assert!(
        ladder.converted(true) < ladder.converted(false),
        "later years deflate below their stated amounts"
    );
    // Zero inflation keeps the bases equal.
    let flat = searched(&plan_from(BASE), &options(), 0.12);
    assert_eq!(flat.converted(true), flat.converted(false));
}

#[test]
fn optimizer_is_deterministic() {
    let plan = plan_from(BASE);
    let first = optimize_conversions(&plan, &TaxTables::embedded(), &options(), 0.12).unwrap();
    let second = optimize_conversions(&plan, &TaxTables::embedded(), &options(), 0.12).unwrap();
    assert_eq!(first.ladder.steps, second.ladder.steps);
    let overlay = |ladder: &OptimizedLadder| {
        ladder_overlay("base.toml", &plan, &options(), &ladder.ladder.steps).unwrap()
    };
    assert_eq!(overlay(&first), overlay(&second));
}

#[test]
fn sweep_covers_every_fillable_bracket() {
    let sweep = sweep_brackets(&plan_from(BASE), &TaxTables::embedded(), &options()).unwrap();
    let params = TaxTables::embedded().params_for(2026, &Inflation::constant(0.0));
    let brackets = params
        .brackets
        .for_status(retiretui_engine::plan::FilingStatus::Single);
    assert_eq!(sweep.brackets.len(), brackets.len() - 1);
    assert!(
        sweep
            .brackets
            .windows(2)
            .all(|pair| pair[0].rate < pair[1].rate),
        "ascending rates"
    );
    let twelve = sweep
        .brackets
        .iter()
        .find(|bracket| (bracket.rate - 0.12).abs() < 1e-9)
        .unwrap();
    assert!(!twelve.steps.is_empty());
    assert!(!sweep.baseline.years.is_empty());
}

#[test]
fn bad_options_are_refused() {
    let refuse = |mutate: fn(&mut OptimizeOptions)| {
        let mut bad = options();
        mutate(&mut bad);
        optimize_conversions(&plan_from(BASE), &TaxTables::embedded(), &bad, 0.12).unwrap_err()
    };
    let params = TaxTables::embedded().params_for(2026, &Inflation::constant(0.0));
    let top_rate = params
        .brackets
        .for_status(retiretui_engine::plan::FilingStatus::Single)
        .last()
        .unwrap()
        .rate;
    let plan = plan_from(BASE);
    let issues = optimize_conversions(&plan, &TaxTables::embedded(), &options(), 0.99).unwrap_err();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("no bracket"))
    );
    let issues =
        optimize_conversions(&plan, &TaxTables::embedded(), &options(), top_rate).unwrap_err();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("top bracket"))
    );
    let issues = refuse(|bad| bad.destination = "cash".to_owned());
    assert!(
        issues
            .iter()
            .any(|issue| issue.path.contains("destination"))
    );
    let issues = refuse(|bad| bad.sources = vec!["ghost".to_owned()]);
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("unknown account"))
    );
    let issues = refuse(|bad| bad.sources = Vec::new());
    assert!(issues.iter().any(|issue| issue.path == "options.sources"));
}

#[test]
fn irmaa_tier_zero_holds_magi_at_the_first_threshold() {
    let with_medicare = BASE.replace("[household]", "[medicare]\n\n[household]");
    let plan = plan_from(&with_medicare);
    let mut capped = options();
    capped.start_year = Some(2043);
    capped.irmaa_tier = Some(0);
    // Fill a high bracket so only the IRMAA ceiling binds.
    let ladder = searched(&plan, &capped, 0.24);
    let first_threshold = TaxTables::embedded()
        .params_for(2026, &Inflation::constant(0.0))
        .irmaa[0]
        .magi_over
        .single;
    let year_2043 = ladder
        .optimized
        .years
        .iter()
        .find(|row| row.year == 2043)
        .unwrap();
    assert_eq!(
        year_2043.taxes.magi, first_threshold,
        "fills to the tier edge"
    );
    assert!(year_2043.conversions > 0);
    assert_eq!(
        ladder.optimized.summary(false).lifetime_medicare,
        0,
        "no surcharge is ever bought"
    );
}

#[test]
fn max_magi_and_cliffs_cap_the_fill() {
    let mut explicit = options();
    explicit.start_year = Some(2041);
    explicit.max_magi = Some(30_000);
    let ladder =
        optimize_conversions(&plan_from(BASE), &TaxTables::embedded(), &explicit, 0.12).unwrap();
    let year_2041 = ladder
        .ladder
        .optimized
        .years
        .iter()
        .find(|row| row.year == 2041)
        .unwrap();
    assert_eq!(year_2041.taxes.magi, 30_000);

    let cliffed = BASE.replace(
        "[[expenses]]",
        "[[cliffs]]\nid = \"aca\"\nmagi_over = 30000\ncost = 12000\ncola = false\n\n[[expenses]]",
    );
    let plan = plan_from(&cliffed);
    let mut windowed = options();
    windowed.start_year = Some(2041);
    let ladder = optimize_conversions(&plan, &TaxTables::embedded(), &windowed, 0.12).unwrap();
    let magi_in = |year: i16| {
        ladder
            .ladder
            .optimized
            .years
            .iter()
            .find(|row| row.year == year)
            .unwrap()
            .taxes
            .magi
    };
    // The default cliff window runs through 2044 (age 65 in 2045).
    assert_eq!(magi_in(2041), 30_000, "capped inside the window");
    assert!(magi_in(2046) > 30_000, "released after the window");
    // The base plan's salary years cross the cliff on their own; the
    // ladder must not add a single further crossing.
    assert_eq!(
        ladder.ladder.optimized.summary(false).lifetime_medicare,
        ladder.baseline.summary(false).lifetime_medicare,
    );
}

#[test]
fn ceiling_options_are_validated() {
    let mut no_medicare = options();
    no_medicare.irmaa_tier = Some(0);
    let issues = optimize_conversions(&plan_from(BASE), &TaxTables::embedded(), &no_medicare, 0.12)
        .unwrap_err();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("[medicare]"))
    );

    let with_medicare = BASE.replace("[household]", "[medicare]\n\n[household]");
    let plan = Plan::from_toml_str(&with_medicare).unwrap();
    let mut big_tier = options();
    big_tier.irmaa_tier = Some(9);
    let issues = optimize_conversions(&plan, &TaxTables::embedded(), &big_tier, 0.12).unwrap_err();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("tiers exist"))
    );
}
