//! How the ledger sweeps, grows and reconciles each account's year.

mod common;

use std::collections::BTreeMap;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::{Action, project};

use common::{FULL, head, run};

#[test]
fn surplus_sweeps_into_cash() {
    let plan = head(
        r#"
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

[[expenses]]
id = "living"
amount = 50000
"#,
    );
    let projection = run(&plan);
    let first = &projection.years[0];
    // Taxable 83,900 -> 1,240 + 4,560 + 7,370 = 13,170.
    assert_eq!(first.taxes.ordinary_taxable, 83_900);
    assert_eq!(first.taxes.ordinary, 13_170);
    assert_eq!(first.surplus, 100_000 - 50_000 - 13_170);
    assert_eq!(first.balances["cash"], first.surplus);
    assert_eq!(first.unfunded, 0);
}

#[test]
fn growth_applies_to_start_balance_only() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "brokerage"
kind = "brokerage"
owner = "me"
balance = 100000
basis = 100000
expected_return = 0.10

[[contributions]]
id = "contribution-1"
to = "brokerage"
amount = 10000

[[income]]
id = "income-2"
kind = "salary"
owner = "me"
amount = 100000
"#,
    );
    let projection = run(&plan);
    let first = &projection.years[0];
    // 100,000 grows 10%; the year's 10,000 contribution earns nothing yet.
    assert_eq!(first.balances["brokerage"], 120_000);
}

#[test]
fn an_account_emptied_this_year_keeps_no_growth() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 100000

[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 10000
expected_return = 0.10

[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "me"
balance = 0
expected_return = 0.10

[[conversions]]
id = "conversion-1"
from = "ira"
to = "roth"
amount = 1000000
on = { date = 2026-01-01 }
cola = false
"#,
    );
    let projection = run(&plan);
    let (first, second) = (&projection.years[0], &projection.years[1]);
    assert_eq!(first.conversions, 11_000, "the year's growth goes too");
    assert_eq!(first.balances["ira"], 0);
    assert_eq!(first.balances["roth"], 11_000);
    assert_eq!(second.balances["ira"], 0, "nothing left to grow");
}

/// Last close, the year's growth and flows, and nothing else, make each
/// account's close.
#[test]
fn every_account_reconciles_from_its_flows_and_growth() {
    let plan = Plan::from_toml_str(FULL).unwrap();
    let projection = project(&plan, &TaxTables::embedded());
    let mut open: BTreeMap<&str, Dollars> = plan
        .accounts
        .iter()
        .map(|account| (account.id.as_str(), account.balance))
        .collect();
    for row in &projection.years {
        let mut expected = open.clone();
        for (id, growth) in &row.growth {
            *expected.get_mut(id.as_str()).unwrap() += growth;
        }
        for (id, taken) in &row.withdrawals {
            *expected.get_mut(id.as_str()).unwrap() -= taken;
        }
        for action in &row.actions {
            match action {
                Action::Transfer { from, to, amount } | Action::Conversion { from, to, amount } => {
                    *expected.get_mut(from.as_str()).unwrap() -= amount;
                    *expected.get_mut(to.as_str()).unwrap() += amount;
                }
                Action::Contribution {
                    account,
                    employee,
                    employer,
                    ..
                } => *expected.get_mut(account.as_str()).unwrap() += employee + employer,
                Action::Surplus { account, amount } => {
                    *expected.get_mut(account.as_str()).unwrap() += amount;
                }
                Action::Rmd { .. } | Action::Withdrawal { .. } => {}
            }
        }
        for (id, close) in &row.balances {
            assert_eq!(expected[id.as_str()], *close, "{} {id}", row.year);
        }
        open = row
            .balances
            .iter()
            .map(|(id, &close)| (id.as_str(), close))
            .collect();
    }
}
