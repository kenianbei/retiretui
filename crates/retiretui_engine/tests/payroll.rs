//! The payroll tax a salary pays, what its pre-tax deductions leave out of
//! everything, what a salary that pays into no Social Security leaves
//! unpaid and unrecorded, and the self-employment tax.

mod common;

use retiretui_engine::optimize::career_at_salary;
use retiretui_engine::params::{Inflation, PayrollParams, TaxTables};
use retiretui_engine::plan::FilingStatus;
use retiretui_engine::tax::{Earnings, Payroll, earnings_at_wage, payroll};

use common::{
    assert_issue, benefit_params, head, issues, params_2026, plan_from, run, with_override,
};

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
        self_employed: 0,
    }
}

fn paid(
    params: &retiretui_engine::params::TaxParams,
    status: FilingStatus,
    earnings: &[Earnings],
) -> i64 {
    payroll(params, status, earnings).total
}

#[test]
fn fica_is_the_employee_shares_to_the_wage_base_and_more_over_the_threshold() {
    let params = params_2026();
    // 6.2% and 1.45% of 60,000.
    assert_eq!(paid(&params, FilingStatus::Single, &[wages(60_000)]), 4_590);
    // OASDI capped at 184,500: 11,439; Medicare on all 300,000: 4,350; 0.9%
    // on the 50,000 over 250,000: 450.
    assert_eq!(
        paid(&params, FilingStatus::MarriedJoint, &[wages(300_000)]),
        16_239
    );
    // Each earner under the base, the threshold on both together.
    assert_eq!(
        paid(
            &params,
            FilingStatus::MarriedJoint,
            &[wages(150_000), wages(150_000)]
        ),
        2 * 11_475 + 450
    );
}

#[test]
fn self_employment_tax_is_twice_the_shares_over_the_wages() {
    let params = params_2026();
    let own = Earnings {
        self_employed: 92_350,
        ..Earnings::default()
    };
    // 12.4% and 2.9% of it: 11,451.40 + 2,678.15.
    assert_eq!(
        payroll(&params, FilingStatus::Single, &[own]),
        Payroll {
            total: 14_130,
            deduction: 7_065,
        }
    );
    let beside_wages = Earnings {
        self_employed: 92_350,
        ..wages(150_000)
    };
    // The wages' 11,475; 12.4% on the 34,500 of base left: 4,278; 2.9% on
    // 92,350: 2,678.15; 0.9% on 42,350 over 200,000: 381.15.
    assert_eq!(paid(&params, FilingStatus::Single, &[beside_wages]), 18_812);
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

#[test]
fn pretax_deductions_leave_cash_wages_the_base_and_the_record() {
    let plan = salaried(60_000, "pretax = 6000");
    let first = &run(&plan).years[0];
    assert_eq!(first.pretax, 6_000);
    assert_eq!(first.income["pay"], 60_000);
    // 54,000 of wages: 7.65% of them, and 37,900 taxable at 10% and 12%.
    assert_eq!(first.taxes.payroll, 4_131);
    assert_eq!(first.taxes.ordinary_taxable, 37_900);
    assert_eq!(first.taxes.ordinary, 4_300);
    assert_eq!(first.surplus, 54_000 - 4_300 - 4_131);
    assert_eq!(
        career_at_salary(&plan_from(&plan), &TaxTables::embedded(), "me").unwrap(),
        earnings_at_wage(&benefit_params(), 54_000, 2026, 2002..=2025)
    );
    let pension = head(&format!(
        "{CASH}\n[[income]]\nid = \"db\"\nkind = \"pension\"\nowner = \"me\"\namount = 20000\npretax = 100\n"
    ));
    assert_issue(
        &issues(&pension),
        "income[0].pretax",
        "only a salary has deductions taken before tax",
    );
    assert_issue(
        &issues(&salaried(60_000, "pretax = 60001")),
        "income[0].pretax",
        "must not exceed the amount",
    );
}

#[test]
fn self_employment_pays_its_tax_half_deducted_and_builds_a_record() {
    let plan = head(&format!(
        "{CASH}\n[[income]]\nid = \"pay\"\nkind = \"self-employment\"\nowner = \"me\"\namount = 100000\ncovered = true\n"
    ));
    assert_eq!(issues(&plan), []);
    let first = &run(&plan).years[0];
    assert_eq!(first.taxes.payroll, 14_130);
    // 100,000 less half the tax, 7,065, less the 16,100 deduction.
    assert_eq!(first.taxes.ordinary_taxable, 76_835);
    assert_eq!(first.taxes.magi, 92_935);
    assert_eq!(
        career_at_salary(&plan_from(&plan), &TaxTables::embedded(), "me").unwrap(),
        earnings_at_wage(&benefit_params(), 92_350, 2026, 2002..=2025)
    );
}
