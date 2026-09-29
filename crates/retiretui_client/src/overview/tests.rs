//! The plans the Overview's rows are tested on.

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

use crate::session::Projected;

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

/// `plan_text` projected against the embedded tables, valid or not.
pub fn projected_from(plan_text: &str) -> Projected {
    Projected::new(
        Plan::from_toml_str(plan_text).unwrap(),
        &TaxTables::embedded(),
    )
}

/// [`TEST_PLAN`] projected.
pub fn test_projected() -> Projected {
    let projected = projected_from(TEST_PLAN);
    assert_eq!(projected.plan.validate(), []);
    projected
}
