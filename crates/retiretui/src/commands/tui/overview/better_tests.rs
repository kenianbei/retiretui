use bevy_app::App;
use plurimus::term::KeyCode;
use retiretui_engine::optimize::{
    ClaimSearch, OptimizeOptions, SweptBracket, apply_claims, apply_ladder, optimize_claims,
    rank_key, sweep_brackets,
};
use retiretui_engine::plan::Plan;
use retiretui_engine::project::Projection;

use super::better::Better;
use super::tests::hold;
use crate::commands::tui::edit::Draft;
use crate::commands::tui::nav::{ActivePage, Page};
use crate::commands::tui::present::{compact_dollars, signed_money};
use crate::commands::tui::session::{Projected, Session};
use crate::commands::tui::support::{
    Headless, SIZE, TEST_PLAN, active_page, commit_edit, press_key, redrawn, scratch_plan,
    searched_app, show,
};
use crate::commands::tui::tools::ladders::tests::table_rows;
use crate::commands::tui::tools::ladders::{self, rate_label};
use crate::commands::tui::tools::{Claims, settle_all};

/// Two people, each with a 401(k) and a Roth IRA, the second named.
const ROTH_OWNERS: &str = r#"
schema = 1

[plan]
name = "roth-owners"
start_year = 2026
horizon_age = 80
inflation = 0.025

[household]
filing = "married-joint"

[[household.people]]
id = "me"
birth = 1966-06-15

[[household.people]]
id = "you"
name = "Sam"
birth = 1968-03-01

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 50000

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 600000
expected_return = 0.05

[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "me"
balance = 10000
expected_return = 0.05

[[accounts]]
id = "you-k"
kind = "401k"
owner = "you"
balance = 400000
expected_return = 0.05

[[accounts]]
id = "you-roth"
kind = "ira"
roth = true
owner = "you"
balance = 10000
expected_return = 0.05

[[expenses]]
id = "living"
amount = 70000
"#;

/// The test plan claiming a computed benefit at 70.
fn claiming() -> String {
    format!(
        "{TEST_PLAN}
[[income]]
id = \"ss\"
kind = \"social-security\"
owner = \"me\"
start = {{ age = 70, owner = \"me\" }}
"
    )
}

fn projected(app: &App) -> &Projected {
    app.world().resource::<Projected>()
}

fn nominal(app: &App) -> bool {
    app.world()
        .resource::<crate::commands::tui::session::Basis>()
        .nominal
}

/// The best ladder into `destination`, searched as the Roth Conversions
/// page searches under the answers the draft holds.
fn best_ladder(app: &App, destination: &str) -> SweptBracket {
    let plan = &projected(app).plan;
    let tables = &app.world().resource::<Session>().tables;
    let (options, rate) = options_of(app, destination);
    let sweep = ladders::search(plan, tables, &options, rate).unwrap();
    sweep.brackets.into_iter().next().unwrap()
}

fn options_of(app: &App, destination: &str) -> (OptimizeOptions, Option<f64>) {
    let held = ladders::held_answers(app.world().resource::<Draft>());
    ladders::options_into(&projected(app).plan, &held, destination).unwrap()
}

fn claim_search(app: &App) -> ClaimSearch {
    let plan = &projected(app).plan;
    let tables = &app.world().resource::<Session>().tables;
    optimize_claims(plan, tables, &[], &[]).unwrap()
}

/// What `option` ends with against the plan as it stands, in the basis
/// shown.
fn ends(app: &App, option: &Projection) -> String {
    let deflated = !nominal(app);
    let own = option.summary(deflated).final_net_worth;
    let current = projected(app).projection.summary(deflated).final_net_worth;
    format!("ends {}", signed_money(own - current))
}

fn takes(app: &mut App, ladder: &SweptBracket, destination: &str) {
    let (options, _) = options_of(app, destination);
    let steps = ladder.steps.clone();
    commit_edit(app, move |plan: &mut Plan| {
        apply_ladder(plan, &options, &steps);
    });
    settle_all(app);
}

#[test]
fn each_roth_owner_is_swept_into_their_own_roth() {
    let mut app = searched_app(scratch_plan(), ROTH_OWNERS, SIZE);
    let frame = redrawn(&mut app);
    for (name, destination) in [("me", "roth"), ("Sam", "you-roth")] {
        let best = best_ladder(&app, destination);
        let row = format!(
            "{name}: convert to {}, {}",
            rate_label(best.rate),
            ends(&app, &best.optimized)
        );
        assert!(frame.contains(&row), "{row}: {frame}");
    }
}

/// A plan holding a ladder other than the best is measured as it stands,
/// and ⏎ lands on the Roth Conversions page on the same search, its best
/// row the one the Overview read.
#[test]
fn enter_on_a_ladder_opens_its_search_with_the_same_best() {
    let mut app = searched_app(scratch_plan(), ROTH_OWNERS, SIZE);
    let tables = app.world().resource::<Session>().tables.clone();
    let plan = projected(&app).plan.clone();
    let (options, _) = options_of(&app, "roth");
    let sweep = sweep_brackets(&plan, &tables, &options).unwrap();
    let best = best_ladder(&app, "roth");
    let lesser = sweep.brackets.iter().rfind(|held| held.steps != best.steps);
    takes(&mut app, &lesser.unwrap().clone(), "roth");
    assert!(!projected(&app).plan.conversions.is_empty());
    assert_eq!(
        best_ladder(&app, "roth").steps,
        best.steps,
        "searched in its place"
    );
    let row = format!("me: convert to {}", rate_label(best.rate));
    hold(&mut app, "Could do better");
    let frame = redrawn(&mut app);
    assert!(
        frame.contains(&format!("{row}, {}", ends(&app, &best.optimized))),
        "{frame}"
    );
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::RothConversions);
    assert_page_best(&mut app, &best);
    let answers = &app.world().resource::<Draft>().tools["optimizer"];
    assert_eq!(
        answers.to_string(),
        "{ to = \"roth\" }",
        "only the destination is named"
    );
}

/// The Roth Conversions page ranks `best` first, as the Overview did.
fn assert_page_best(app: &mut Headless, best: &SweptBracket) {
    settle_all(app);
    let frame = redrawn(app);
    let rows = table_rows(&frame);
    let first = &rows
        .get(1)
        .unwrap_or_else(|| panic!("no option: {frame}"))
        .1;
    let converted = best.optimized.summary(!nominal(app)).lifetime_conversions;
    let expected = [rate_label(best.rate), compact_dollars(converted)];
    assert_eq!(first[..2], expected, "{frame}");
}

/// The full fixture's one Roth account.
const ROTH: &str = "roth-ira";

/// The Overview searches under the constraints the form holds, and ⏎
/// names the destination beside them: a held bracket other than the
/// best that still beats the plan.
#[test]
fn a_held_constraint_is_searched_under_and_kept() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../retiretui_engine/tests/fixtures/full.toml"
    );
    let mut app = searched_app(
        scratch_plan(),
        &std::fs::read_to_string(fixture).unwrap(),
        SIZE,
    );
    let current = rank_key(&projected(&app).projection);
    let (options, _) = options_of(&app, ROTH);
    let tables = &app.world().resource::<Session>().tables;
    let sweep = ladders::search(&projected(&app).plan, tables, &options, None).unwrap();
    let held = (sweep.brackets[1..].iter())
        .find(|bracket| rank_key(&bracket.optimized) < current)
        .expect("a lesser ladder beats the plan");
    let percent = (held.rate * 100.0).round() as i64;
    let mut answers = toml::Table::new();
    answers.insert("bracket".to_owned(), percent.into());
    app.world_mut()
        .resource_mut::<Draft>()
        .tools
        .insert("optimizer".to_owned(), toml::Value::Table(answers));
    settle_all(&mut app);
    let best = best_ladder(&app, ROTH);
    assert_eq!(rate_label(best.rate), format!("{percent}%"));
    hold(&mut app, "Could do better");
    let row = format!(
        "jordan: convert to {percent}%, {}",
        ends(&app, &best.optimized)
    );
    let frame = redrawn(&mut app);
    assert!(frame.contains(&row), "{row}: {frame}");
    press_key(&mut app, KeyCode::Enter);
    assert_page_best(&mut app, &best);
    let answers = &app.world().resource::<Draft>().tools["optimizer"];
    assert_eq!(answers["bracket"].as_integer(), Some(percent));
    assert_eq!(answers["to"].as_str(), Some(ROTH));
}

/// A held source only one owner's search accepts keeps the other's row,
/// saying so, and ⏎ on it still aims the page at their Roth.
#[test]
fn an_owner_refused_under_the_held_answers_keeps_a_row() {
    let mut app = searched_app(scratch_plan(), ROTH_OWNERS, SIZE);
    let mut answers = toml::Table::new();
    answers.insert("from".to_owned(), "k".into());
    app.world_mut()
        .resource_mut::<Draft>()
        .tools
        .insert("optimizer".to_owned(), toml::Value::Table(answers));
    settle_all(&mut app);
    hold(&mut app, "Could do better");
    let frame = redrawn(&mut app);
    assert!(frame.contains("me: convert to "), "{frame}");
    let refused = "Sam: not searchable under the Roth Conversions answers";
    assert!(frame.contains(refused), "{frame}");
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::RothConversions);
    let answers = &app.world().resource::<Draft>().tools["optimizer"];
    assert_eq!(answers["to"].as_str(), Some("you-roth"));
}

/// The sweep searches in place of the plan's own ladder, so a plan that
/// already holds the best one has nothing to gain.
#[test]
fn a_plan_holding_the_best_ladder_shows_no_gain() {
    let mut app = searched_app(scratch_plan(), ROTH_OWNERS, SIZE);
    let best = best_ladder(&app, "roth");
    takes(&mut app, &best, "roth");
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("me: no conversion ladder beats the plan"),
        "{frame}"
    );
}

#[test]
fn the_claim_search_shows_its_best_and_enter_opens_it() {
    let mut app = searched_app(scratch_plan(), &claiming(), SIZE);
    let search = claim_search(&app);
    let best = search.best();
    let row = format!(
        "Claim me at {}: {}",
        best.claims[0].age,
        ends(&app, &best.projection)
    );
    hold(&mut app, "Could do better");
    let frame = redrawn(&mut app);
    assert!(frame.contains(&row), "{row}: {frame}");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::SsaBenefits);
    settle_all(&mut app);
    let claims = app.world().resource::<Claims>();
    let shown = claims.found().expect("the page searched").best();
    assert_eq!(shown.claims, best.claims);
    assert_eq!(
        shown.projection.summary(true),
        best.projection.summary(true)
    );
}

#[test]
fn claims_already_at_their_best_say_so() {
    let mut app = searched_app(scratch_plan(), &claiming(), SIZE);
    let search = claim_search(&app);
    let (added, claims) = (search.added.clone(), search.best().claims.clone());
    commit_edit(&mut app, move |plan: &mut Plan| {
        apply_claims(plan, &added, &claims);
    });
    settle_all(&mut app);
    let frame = redrawn(&mut app);
    assert!(frame.contains("Claims as planned are best"), "{frame}");
}

#[test]
fn a_plan_with_nothing_to_search_says_so() {
    let mut app = searched_app(scratch_plan(), TEST_PLAN, SIZE);
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("No conversion or claim to search"),
        "{frame}"
    );
}

#[test]
fn the_searches_run_only_while_the_overview_is_shown() {
    let mut app = searched_app(scratch_plan(), ROTH_OWNERS, SIZE);
    let is_running = |app: &App| app.world().resource::<Better>().is_running();
    commit_edit(&mut app, |plan: &mut Plan| plan.expenses[0].amount = 50_000);
    app.update();
    assert!(is_running(&app), "a changed plan is searched again");
    app.insert_resource(ActivePage(Page::Ledger));
    app.update();
    assert!(!is_running(&app), "leaving the page stops the search");
    commit_edit(&mut app, |plan: &mut Plan| plan.expenses[0].amount = 55_000);
    app.update();
    assert!(!is_running(&app), "nor does another page start one");
    show(&mut app, Page::Overview);
    settle_all(&mut app);
    show(&mut app, Page::Ledger);
    show(&mut app, Page::Overview);
    assert!(!is_running(&app), "the answer is kept for its plan");
    assert!(app.world().resource::<Better>().found().is_some());
}

#[test]
fn a_failing_historical_start_leads_to_the_historical_page() {
    let plan = TEST_PLAN.replace("amount = 60000", "amount = 90000");
    let mut app = searched_app(scratch_plan(), &plan, SIZE);
    hold(&mut app, "Needs attention");
    let frame = redrawn(&mut app);
    assert!(frame.contains("▌ Fails from a "), "{frame}");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Historical);
}
