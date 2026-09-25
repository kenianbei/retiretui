//! Locked accounts: what waits for an unlock and what fires once it comes.

mod common;

use retiretui_engine::project::Action;

use common::{head, run};

#[test]
fn conversions_skip_locked_years() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 100000

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 50000
locked_until = { date = 2028-01-01 }

[[accounts]]
id = "r"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[conversions]]
id = "conversion-2"
from = "k"
to = "r"
amount = 10000
cola = false

[[expenses]]
id = "living"
amount = 20000
"#,
    );
    let projection = run(&plan);
    assert_eq!(projection.years[0].conversions, 0, "2026 locked");
    assert_eq!(projection.years[1].conversions, 0, "2027 locked");
    assert_eq!(projection.years[2].conversions, 10_000, "2028 unlocked");
    assert_eq!(projection.years[2].balances["r"], 10_000);
}

#[test]
fn one_shot_conversions_defer_until_the_source_unlocks() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 100000

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 50000
locked_until = { date = 2028-01-01 }

[[accounts]]
id = "r"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[conversions]]
id = "conversion-3"
from = "k"
to = "r"
amount = 10000
on = { date = 2026-06-01 }
cola = false

[[expenses]]
id = "living"
amount = 20000
"#,
    );
    let projection = run(&plan);
    assert_eq!(projection.years[0].conversions, 0, "2026 locked");
    assert_eq!(projection.years[1].conversions, 0, "2027 locked");
    assert_eq!(projection.years[2].conversions, 10_000, "fires at unlock");
    assert_eq!(projection.years[3].conversions, 0, "fires once");
}

#[test]
fn transfers_defer_until_the_source_unlocks() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 100000

[[accounts]]
id = "pension-dc"
kind = "414k"
owner = "me"
balance = 60000
locked_until = { date = 2028-01-01 }

[[transfers]]
id = "transfer-1"
from = "pension-dc"
to = "cash"
on = { date = 2026-06-01 }

[[expenses]]
id = "living"
amount = 20000
"#,
    );
    let projection = run(&plan);
    assert_eq!(
        projection.years[0].balances["pension-dc"], 60_000,
        "2026 locked"
    );
    assert_eq!(
        projection.years[1].balances["pension-dc"], 60_000,
        "2027 locked"
    );
    assert_eq!(projection.years[2].balances["pension-dc"], 0, "2028 fires");
    // Deferred to taxable is a distribution: the whole move is ordinary
    // income in the unlock year (less that year's indexed deduction).
    assert_eq!(projection.years[0].taxes.ordinary_taxable, 0);
    assert!(projection.years[2].taxes.ordinary_taxable > 40_000);
    assert!(
        !projection.years[0]
            .actions
            .iter()
            .any(|action| matches!(action, Action::Transfer { .. }))
    );
    assert!(projection.years[2].actions.contains(&Action::Transfer {
        from: "pension-dc".into(),
        to: "cash".into(),
        amount: 60_000,
    }));
}

#[test]
fn locked_account_is_skipped_until_transfer_unlocks_it() {
    let plan = head(
        r#"
[[events]]
id = "retire"
trigger = { age = 50, owner = "me" }

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 200000

[[accounts]]
id = "pension-dc"
kind = "414k"
owner = "me"
balance = 100000
locked_until = { income = "pension" }

[[accounts]]
id = "rollover"
kind = "ira"
owner = "me"
balance = 0

[[income]]
id = "pension"
kind = "pension"
owner = "me"
amount = 12000
start = { event = "retire" }

[[expenses]]
id = "living"
amount = 30000

[[transfers]]
id = "transfer-2"
from = "pension-dc"
to = "rollover"
on = { income = "pension" }
"#,
    );
    let projection = run(&plan);
    let first = &projection.years[0];
    assert!(!first.withdrawals.contains_key("pension-dc"));
    assert_eq!(first.balances["pension-dc"], 100_000);
    // 2030 is the retire/pension year: the DC account rolls over whole.
    let unlock = projection
        .years
        .iter()
        .find(|row| row.year == 2030)
        .unwrap();
    assert_eq!(unlock.balances["pension-dc"], 0);
    assert!(unlock.balances["rollover"] >= 100_000);
    assert!(unlock.income.contains_key("pension"));
}
