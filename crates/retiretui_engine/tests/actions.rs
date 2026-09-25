//! The actions each projected year records.

mod common;

use retiretui_engine::project::Action;

use common::{head, run};

/// A contribution and a conversion clamped to its source's balance.
const ACTIONS_PLAN: &str = r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 100000

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 6000

[[accounts]]
id = "r"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[contributions]]
id = "contribution-2"
to = "r"
amount = 7000

[[conversions]]
id = "conversion-7"
from = "k"
to = "r"
amount = 10000
cola = false
"#;

#[test]
fn actions_record_executed_amounts_in_order() {
    let plan = head(ACTIONS_PLAN);
    let first = &run(&plan).years[0];
    // The conversion is clamped to the 6,000 balance; the contribution
    // executes as stated. Order follows the step order.
    assert_eq!(
        first.actions,
        vec![
            Action::Contribution {
                account: "r".into(),
                employee: 7_000,
                employer: 0,
                notes: vec![],
            },
            Action::Conversion {
                from: "k".into(),
                to: "r".into(),
                amount: 6_000,
            },
            Action::Withdrawal {
                account: "cash".into(),
                amount: 7_000,
            },
        ],
    );
}

#[test]
fn actions_split_rmds_from_funding_withdrawals() {
    let plan = r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 80
inflation = 0.025

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1950-06-15

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 500000

[[expenses]]
id = "living"
amount = 60000
"#;
    let first = &run(plan).years[0];
    let rmd = first.rmds;
    assert!(rmd > 0);
    let funding = first.withdrawals["k"] - rmd;
    assert!(funding > 0, "expenses exceed the RMD");
    assert_eq!(
        first.actions,
        vec![
            Action::Rmd {
                account: "k".into(),
                amount: rmd,
            },
            Action::Withdrawal {
                account: "k".into(),
                amount: funding,
            },
        ],
    );
}
