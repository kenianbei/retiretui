//! The payroll tax a salary pays, and what a salary that pays into no
//! Social Security leaves unpaid and unrecorded.

mod common;

use retiretui_engine::optimize::career_at_salary;
use retiretui_engine::params::{Inflation, PayrollParams, TaxTables};
use retiretui_engine::plan::FilingStatus;
use retiretui_engine::tax::{Earnings, payroll};

use common::{assert_issue, head, issues, params_2026, plan_from, run, with_override};

const CASH: &str = r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 100000
"#;

fn salaried(amount: u32, more: &str) -> String {
    head(&format!(
        "{CASH}\n[[income]]\nid = \"pay\"\nkind = \"salary\"\nowner = \"me\"\namount = {amount}\n{more}"
    ))
}

fn wages(amount: i64) -> Earnings {
    Earnings {
        covered: amount,
        medicare: amount,
    }
}

#[test]
fn fica_is_the_employee_shares_to_the_wage_base_and_more_over_the_threshold() {
    let params = params_2026();
    // 6.2% and 1.45% of 60,000.
    assert_eq!(
        payroll(&params, FilingStatus::Single, &[wages(60_000)]),
        4_590
    );
    // OASDI capped at 184,500: 11,439; Medicare on all 300,000: 4,350; 0.9%
    // on the 50,000 over 250,000: 450.
    assert_eq!(
        payroll(&params, FilingStatus::MarriedJoint, &[wages(300_000)]),
        16_239
    );
    // Each earner under the base, the threshold on both together.
    assert_eq!(
        payroll(
            &params,
            FilingStatus::MarriedJoint,
            &[wages(150_000), wages(150_000)]
        ),
        2 * 11_475 + 450
    );
    assert_eq!(payroll(&params, FilingStatus::Single, &[wages(-1_000)]), 0);
}

#[test]
fn a_salary_pays_its_payroll_tax_from_cash() {
    let projection = run(&salaried(60_000, ""));
    let first = &projection.years[0];
    assert_eq!(first.taxes.payroll, 4_590);
    assert_eq!(
        first.taxes.total,
        first.taxes.ordinary + first.taxes.state + 4_590
    );
}

#[test]
fn a_payroll_hsa_contribution_leaves_the_base_but_a_deferral_does_not() {
    let hsa = r#"
[[accounts]]
id = "hsa"
kind = "hsa"
owner = "me"
balance = 0

[[contributions]]
id = "to-hsa"
to = "hsa"
amount = 4000
cola = false
"#;
    assert_eq!(run(&salaried(60_000, hsa)).years[0].taxes.payroll, 4_284);
    let deferral = r#"
[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 0

[[contributions]]
id = "to-k"
to = "k"
amount = 10000
cola = false
"#;
    assert_eq!(
        run(&salaried(60_000, deferral)).years[0].taxes.payroll,
        4_590
    );
}

#[test]
fn a_salary_into_no_social_security_pays_medicare_alone_and_builds_no_record() {
    const CLAIM: &str = r#"
end = { age = 66, owner = "me" }

[[income]]
id = "ss"
kind = "social-security"
owner = "me"
start = { age = 67, owner = "me" }
"#;
    let covered = run(&salaried(60_000, CLAIM));
    let uncovered = run(&salaried(
        60_000,
        &format!("social_security = false\n{CLAIM}"),
    ));
    assert_eq!(uncovered.years[0].taxes.payroll, 870);
    let benefit = |projection: &retiretui_engine::project::Projection| {
        projection.row(2048).unwrap().income.get("ss").copied()
    };
    assert!(benefit(&covered).is_some_and(|paid| paid > 0));
    assert_eq!(benefit(&uncovered), None);
    let refused = career_at_salary(
        &plan_from(&salaried(60_000, "social_security = false")),
        &TaxTables::embedded(),
        "me",
    )
    .unwrap_err();
    assert!(
        refused.contains("pays into no Social Security"),
        "{refused}"
    );
}

#[test]
fn only_a_salary_can_pay_into_no_social_security() {
    let pension = head(&format!(
        "{CASH}\n[[income]]\nid = \"db\"\nkind = \"pension\"\nowner = \"me\"\namount = 20000\nsocial_security = false\n"
    ));
    assert_issue(
        &issues(&pension),
        "income[0].social_security",
        "only a salary can pay into no Social Security",
    );
}

#[test]
fn the_embedded_rates_are_the_statutory_default_a_table_may_leave_out() {
    assert_eq!(params_2026().payroll, PayrollParams::default());
    let overridden = with_override().params_for(2027, &Inflation::constant(0.0));
    assert_eq!(overridden.payroll, PayrollParams::default());
    let later = TaxTables::embedded().params_for(2040, &Inflation::constant(0.03));
    assert_eq!(later.payroll, PayrollParams::default());
}
