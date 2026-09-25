//! Contributions held to the law's limits, and the basis they leave.

mod common;

use retiretui_engine::params::{Inflation, TaxTables};
use retiretui_engine::plan::{AccountKind, FilingStatus};
use retiretui_engine::project::{Action, ContributionNote, Projection};
use retiretui_engine::tax;

use common::{head, run};

/// The 2026 limit for a single person under fifty, as the table has it.
fn limit_2026(kind: AccountKind) -> i64 {
    let params = TaxTables::embedded().params_for(2026, &Inflation::constant(0.0));
    tax::employee_limit(&params, FilingStatus::Single, kind, 46).unwrap()
}

fn contribution<'a>(
    projection: &'a Projection,
    year: usize,
    account: &str,
) -> (i64, i64, &'a [ContributionNote]) {
    projection.years[year]
        .actions
        .iter()
        .find_map(|action| match action {
            Action::Contribution {
                account: paid,
                employee,
                employer,
                notes,
            } if paid == account => Some((*employee, *employer, notes.as_slice())),
            _ => None,
        })
        .unwrap_or((0, 0, &[]))
}

const ONE_401K: &str = r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "401k"
kind = "401k"
owner = "me"
balance = 0

[[income]]
id = "pay"
kind = "salary"
owner = "me"
amount = 100000
cola = false
"#;

#[test]
fn an_employee_amount_over_the_limit_is_held_to_it_and_says_so() {
    let text = head(&format!(
        "{ONE_401K}\n[[contributions]]\nid = \"contribution-3\"\nto = \"401k\"\namount = 30000\n"
    ));
    let projection = run(&text);
    let (employee, _, notes) = contribution(&projection, 0, "401k");
    assert_eq!(employee, limit_2026(AccountKind::K401k));
    assert_eq!(notes, [ContributionNote::HeldToLimit]);
    let projection = run(&text.replace("amount = 30000", "amount = 24000"));
    let (employee, _, notes) = contribution(&projection, 0, "401k");
    assert_eq!((employee, notes.len()), (24_000, 0));
}

#[test]
fn a_flat_amount_under_the_limit_is_never_held() {
    let text = head(&format!(
        "{ONE_401K}\n[[contributions]]\nid = \"contribution-4\"\nto = \"401k\"\namount = 24500\ncola = false\n"
    ));
    let projection = run(&text);
    for year in 0..projection.years.len() {
        let (employee, _, notes) = contribution(&projection, year, "401k");
        assert_eq!((employee, notes.len()), (24_500, 0), "year {year}");
    }
}

#[test]
fn a_share_of_an_income_follows_its_gross_and_steps_up_to_its_ceiling() {
    let text = head(&format!(
        "{ONE_401K}\n[[contributions]]\nid = \"contribution-5\"\nto = \"401k\"\nrate = 0.06\nof = \"pay\"\nstep = {{ add = 0.01, up_to = 0.08 }}\n"
    ));
    let projection = run(&text);
    let paid: Vec<i64> = (0..4)
        .map(|year| contribution(&projection, year, "401k").0)
        .collect();
    assert_eq!(paid, [6_000, 7_000, 8_000, 8_000]);
    let (_, _, notes) = contribution(&projection, 1, "401k");
    assert_eq!(
        notes,
        [ContributionNote::Share {
            rate: 0.07,
            of: "pay".into()
        }]
    );
}

#[test]
fn two_plans_of_one_person_share_the_limit_in_file_order() {
    let text = head(&format!(
        "{ONE_401K}\n[[accounts]]\nid = \"403b\"\nkind = \"403b\"\nowner = \"me\"\nbalance = 0\n\n[[contributions]]\nid = \"contribution-6\"\nto = \"401k\"\namount = 20000\n\n[[contributions]]\nid = \"contribution-7\"\nto = \"403b\"\namount = 20000\n"
    ));
    let projection = run(&text);
    assert_eq!(contribution(&projection, 0, "401k").0, 20_000);
    let (employee, _, notes) = contribution(&projection, 0, "403b");
    assert_eq!(employee, limit_2026(AccountKind::K401k) - 20_000);
    assert_eq!(notes, [ContributionNote::HeldToLimit]);
}

#[test]
fn an_employer_hsa_amount_takes_the_hsa_room() {
    let text = head(&format!(
        "{ONE_401K}\n[[accounts]]\nid = \"hsa\"\nkind = \"hsa\"\nowner = \"me\"\nbalance = 0\n\n[[contributions]]\nid = \"contribution-8\"\nto = \"hsa\"\namount = 4000\n\n[[contributions]]\nid = \"contribution-9\"\nto = \"hsa\"\nby = \"employer\"\namount = 1000\n"
    ));
    let projection = run(&text);
    let (yours, theirs, notes) = contribution(&projection, 0, "hsa");
    assert_eq!(
        (yours, theirs),
        (4_000, limit_2026(AccountKind::Hsa) - 4_000)
    );
    assert_eq!(notes, [ContributionNote::HeldToLimit]);
}

#[test]
fn the_employer_side_gives_way_under_the_overall_cap() {
    let text = head(&format!(
        "{ONE_401K}\n[[contributions]]\nid = \"contribution-10\"\nto = \"401k\"\namount = 20000\n\n[[contributions]]\nid = \"contribution-11\"\nto = \"401k\"\nby = \"employer\"\namount = 60000\n"
    ));
    let projection = run(&text);
    let (yours, theirs, notes) = contribution(&projection, 0, "401k");
    assert_eq!((yours, theirs), (20_000, 72_000 - 20_000));
    assert_eq!(notes, [ContributionNote::HeldToOverall]);
}

#[test]
fn a_maximum_pays_the_year_s_limit_and_a_match_follows_what_was_deferred() {
    let text = head(&format!(
        "{ONE_401K}\n[[contributions]]\nid = \"contribution-12\"\nto = \"401k\"\nmax = true\n\n[[contributions]]\nid = \"contribution-13\"\nto = \"401k\"\nby = \"employer\"\nof = \"pay\"\nmatch = {{ rate = 0.5, up_to = 0.06 }}\n"
    ));
    let projection = run(&text);
    let (yours, theirs, notes) = contribution(&projection, 0, "401k");
    assert_eq!((yours, theirs), (limit_2026(AccountKind::K401k), 3_000));
    assert_eq!(
        notes,
        [
            ContributionNote::Maximum,
            ContributionNote::Match {
                rate: 0.5,
                up_to: 0.06,
                of: "pay".into()
            }
        ]
    );
    let text = text.replace("max = true", "amount = 4000");
    let projection = run(&text);
    let (yours, theirs, _) = contribution(&projection, 0, "401k");
    assert_eq!((yours, theirs), (4_000, 2_000), "half of a 4,000 deferral");
}

#[test]
fn after_tax_money_fills_what_the_cap_leaves_and_comes_back_untaxed() {
    let text = head(&format!(
        "{ONE_401K}\n[[contributions]]\nid = \"contribution-14\"\nto = \"401k\"\namount = 20000\n\n[[contributions]]\nid = \"contribution-15\"\nto = \"401k\"\nby = \"employer\"\namount = 10000\n\n[[contributions]]\nid = \"contribution-16\"\nto = \"401k\"\nby = \"after-tax\"\namount = 50000\n"
    ));
    let projection = run(&text);
    let (yours, theirs, notes) = contribution(&projection, 0, "401k");
    assert_eq!((yours, theirs), (20_000 + 42_000, 10_000));
    assert_eq!(
        notes,
        [
            ContributionNote::HeldToOverall,
            ContributionNote::AfterTax { amount: 42_000 }
        ]
    );
    assert_eq!(
        projection.years[0].taxes.ordinary_taxable,
        run(&head(&format!(
            "{ONE_401K}\n[[contributions]]\nid = \"contribution-17\"\nto = \"401k\"\namount = 20000\n"
        )))
        .years[0]
            .taxes
            .ordinary_taxable,
        "after-tax money is not deducted"
    );
}

const IRA_DRAWN: &str = r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 300000
expected_return = 0.0
BASIS

[[expenses]]
id = "living"
amount = 60000
"#;

#[test]
fn basis_in_an_ira_comes_back_untaxed_on_a_withdrawal() {
    let untaxed = run(&head(&IRA_DRAWN.replace("BASIS", "basis = 300000")));
    let taxed = run(&head(&IRA_DRAWN.replace("BASIS", "")));
    let (first, other) = (&untaxed.years[0], &taxed.years[0]);
    assert_eq!(first.withdrawals["ira"], 60_000, "nothing grossed up");
    assert_eq!(first.taxes.total, 0);
    assert!(other.taxes.total > 0 && other.withdrawals["ira"] > 60_000);
}

#[test]
fn a_person_s_iras_share_their_basis_pro_rata_whichever_is_drawn() {
    let two = |basis_a: i64, basis_b: i64| {
        head(&format!(
            r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "a"
kind = "ira"
owner = "me"
balance = 150000
basis = {basis_a}
expected_return = 0.0
drain_priority = 9

[[accounts]]
id = "b"
kind = "ira"
owner = "me"
balance = 150000
basis = {basis_b}
expected_return = 0.0
drain_priority = 0

[[expenses]]
id = "living"
amount = 60000
"#
        ))
    };
    let pooled = run(&two(150_000, 0)).years[0].taxes.total;
    let spread = run(&two(75_000, 75_000)).years[0].taxes.total;
    let none = run(&two(0, 0)).years[0].taxes.total;
    assert_eq!(
        pooled, spread,
        "the pool is what counts, not the account drawn"
    );
    assert!(pooled < none);
}

#[test]
fn basis_comes_back_untaxed_on_a_conversion_and_a_required_distribution() {
    let text = head(&format!(
        "{}\n[[accounts]]\nid = \"roth\"\nkind = \"ira\"\nroth = true\nowner = \"me\"\nbalance = 0\n\n[[conversions]]\nid = \"conversion-8\"\nfrom = \"ira\"\nto = \"roth\"\namount = 50000\n",
        IRA_DRAWN.replace("BASIS", "basis = 300000")
    ));
    let converted = run(&text);
    assert_eq!(converted.years[0].conversions, 50_000);
    assert_eq!(converted.years[0].taxes.ordinary_taxable, 0);
    let rmd = |basis: &str| {
        head(&format!(
            "[[accounts]]\nid = \"cash\"\nkind = \"cash\"\nowner = \"me\"\nbalance = 0\n\n[[accounts]]\nid = \"ira\"\nkind = \"ira\"\nowner = \"me\"\nbalance = 1000000\nexpected_return = 0.0\n{basis}\n"
        ))
        .replace("birth = 1980-06-15", "birth = 1950-06-15")
        .replace("horizon_age = 70", "horizon_age = 80")
    };
    let untaxed = run(&rmd("basis = 1000000"));
    let taxed = run(&rmd(""));
    assert!(untaxed.years[0].rmds > 0);
    assert_eq!(untaxed.years[0].taxes.ordinary_taxable, 0);
    assert!(taxed.years[0].taxes.ordinary_taxable > 0);
}

/// A 401(k) deferral, so the person is covered, beside a traditional IRA
/// contribution and a Roth one; the salary sets where the MAGI lands.
fn ira_plan(salary: i64, deferral: i64) -> String {
    head(&format!(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "401k"
kind = "401k"
owner = "me"
balance = 0

[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 0

[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[income]]
id = "pay"
kind = "salary"
owner = "me"
amount = {salary}
cola = false

[[contributions]]
id = "contribution-18"
to = "401k"
amount = {deferral}
cola = false

[[contributions]]
id = "contribution-19"
to = "ira"
amount = 3000
cola = false

[[contributions]]
id = "contribution-20"
to = "roth"
amount = 3000
cola = false
"#
    ))
}

#[test]
fn a_roth_ira_contribution_over_the_income_band_is_said() {
    let over = run(&ira_plan(200_000, 0));
    let (_, _, notes) = contribution(&over, 0, "roth");
    assert_eq!(notes, [ContributionNote::RothIraPhaseOut]);
    let under = run(&ira_plan(100_000, 0));
    assert!(contribution(&under, 0, "roth").2.is_empty());
}

#[test]
fn a_covered_person_s_ira_deduction_phases_out_across_the_band() {
    // MAGI 81,000 is the band's foot: all deducted, nothing to say.
    let foot = run(&ira_plan(95_000, 14_000));
    assert!(contribution(&foot, 0, "ira").2.is_empty());
    // MAGI 86,000 is halfway: half the 3,000 is deducted.
    let middle = run(&ira_plan(100_000, 14_000));
    assert_eq!(
        contribution(&middle, 0, "ira").2,
        [ContributionNote::NotDeducted { amount: 1_500 }]
    );
    assert_eq!(
        middle.years[0].taxes.ordinary_taxable,
        foot.years[0].taxes.ordinary_taxable + 5_000 + 1_500,
        "5,000 more salary, and 1,500 less deducted"
    );
    // MAGI 96,000 is past the top: none deducted.
    let top = run(&ira_plan(110_000, 14_000));
    assert_eq!(
        contribution(&top, 0, "ira").2,
        [ContributionNote::NotDeducted { amount: 3_000 }]
    );
    // Uncovered, the same MAGI deducts in full.
    let uncovered = run(&ira_plan(110_000, 0));
    assert!(contribution(&uncovered, 0, "ira").2.is_empty());
}
