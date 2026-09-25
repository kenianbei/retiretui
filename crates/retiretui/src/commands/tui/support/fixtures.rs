//! The plans the tests run on, projected or written to scratch files.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::project;

use crate::commands::tui::session::Projected;

pub const TEST_PLAN: &str = r#"
schema = 1

[plan]
name = "test-plan"
start_year = 2026
horizon_age = 70
inflation = 0.025

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1980-06-15

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 10000

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 200000
expected_return = 0.05

[[income]]
id = "salary"
kind = "salary"
owner = "me"
amount = 100000
end = { age = 60, owner = "me" }

[[expenses]]
id = "living"
amount = 60000
"#;

/// Appended to a plan, runs its Monte Carlo through few enough markets
/// that a test waits on it briefly.
pub const FEW_TRIALS: &str = "\n[market.monte_carlo]\ntrials = 20\n";

/// [`TEST_PLAN`], its Monte Carlo cut to [`FEW_TRIALS`].
pub fn test_plan_briefly_run() -> String {
    format!("{TEST_PLAN}{FEW_TRIALS}")
}

/// A small deterministic plan projected against the embedded tables.
pub fn test_projected() -> Projected {
    let projected = projected_from(TEST_PLAN);
    let issues = projected.plan.validate();
    assert!(issues.is_empty(), "invalid test plan: {issues:?}");
    projected
}

/// `plan_text` projected against the embedded tables, valid or not.
pub fn projected_from(plan_text: &str) -> Projected {
    let plan = Plan::from_toml_str(plan_text).unwrap();
    let projection = project(&plan, &TaxTables::embedded());
    Projected { plan, projection }
}

static NEXT_SCRATCH: AtomicUsize = AtomicUsize::new(0);

/// A unique on-disk copy of the test plan, so parallel tests never share a
/// file.
pub fn scratch_plan() -> PathBuf {
    let ordinal = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "retiretui-tui-{}-{ordinal}.toml",
        std::process::id()
    ));
    std::fs::write(&path, TEST_PLAN).unwrap();
    path
}

/// The engine's full fixture, copied to a scratch path of its own.
pub fn scratch_full_plan() -> PathBuf {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../retiretui_engine/tests/fixtures/full.toml");
    let scratch = scratch_plan();
    std::fs::copy(fixture, &scratch).unwrap();
    scratch
}

/// A scenario over the plan file `base` beside it, renaming the plan.
pub fn scenario_over(base: &str) -> String {
    format!("schema = 1\nbase = \"{base}\"\n\n[plan]\nname = \"variant\"\n")
}

/// A scenario file over a scratch copy of the test plan, which opens
/// read-only.
pub fn scratch_scenario() -> PathBuf {
    let base = scratch_plan();
    let scenario = base.with_extension("scenario.toml");
    let overlay = scenario_over(&base.file_name().unwrap().to_string_lossy());
    std::fs::write(&scenario, overlay).unwrap();
    scenario
}

/// A directory of its own holding `plan_text` as plan.toml, a scenario
/// over it, a broken file, and one that is not TOML.
pub fn scratch_workspace(plan_text: &str) -> PathBuf {
    let ordinal = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("retiretui-tui-{}-{ordinal}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("plan.toml"), plan_text).unwrap();
    std::fs::write(dir.join("variant.toml"), scenario_over("plan.toml")).unwrap();
    std::fs::write(dir.join("broken.toml"), "schema = 1\n").unwrap();
    std::fs::write(dir.join("notes.txt"), "not a plan").unwrap();
    dir
}
