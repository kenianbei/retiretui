//! The records behind a selection of the spending estimate and what
//! sorted each: the user's rules, the bank's column and the embedded table.

mod common;

use common::{GROCERIES, proposal_amount, spending_plan};
use retiretui_engine::estimate::{
    By, Category, Of, Outcome, Patterns, Row, estimate, read, sorted_records, spent_a_year,
};

/// The rows `of` selects, as `sorted_records` finds them.
fn records<'a>(rows: &'a [Row], patterns: &Patterns, of: &Of) -> Vec<&'a Row> {
    let found = sorted_records(rows, patterns, of);
    found.into_iter().map(|sorted| sorted.row).collect()
}

#[test]
fn records_are_a_categorys_or_a_payees_rows_in_the_window_newest_first() {
    let csv = "Date,Description,Amount\n2025-01-05,SAFEWAY,-1.00\n2026-09-01,SAFEWAY,-50.00\n2026-10-01,SAFEWAY,-60.00\n2026-10-02,SOMEONE,-30.00\n2026-10-03,ELSEWHERE,-20.00\n";
    let rows = read("x.csv", csv).unwrap();
    let patterns = Patterns::embedded();
    let amounts = |of: &Of| -> Vec<i64> {
        records(&rows, &patterns, of)
            .iter()
            .map(|row| row.amount.0)
            .collect()
    };
    assert_eq!(
        amounts(&Of::Category(Category::Groceries)),
        [-6000, -5000],
        "the row a year before the window is left out"
    );
    assert_eq!(amounts(&Of::Payee("SOMEONE".to_owned())), [-3000]);
    assert_eq!(amounts(&Of::Unnamed), [-2000, -3000]);
    assert!(records(&[], &patterns, &Of::Unnamed).is_empty());
}

#[test]
fn the_users_rules_outrank_the_bank_merge_payees_and_ignore_rows() {
    let csv = "Date,Description,Category,Amount\n2026-10-01,SAFEWAY,Food & Drink,-50.00\n2026-10-02,CHECK PAID,,-1500.00\n2026-10-03,CHECK PAID,,-40.00\n2026-10-04,ACME HELP,,-10.00\n2026-10-05,ACME SHOP,,-20.00\n2026-10-06,REFUNDED THING,,-30.00\n2026-10-07,REFUNDED THING,,-31.00\n2026-10-31,NEVER,,-5.00\n";
    let rules = r#"
patterns = [
  { is = "SAFEWAY", category = "groceries" },
  { is = "CHECK PAID", spent = [1400, 1600], export = "joint", category = "housing" },
  { is = "REFUNDED THING", on = 2026-10-06, spent = [30, 30], ignore = true },
  { is = "NEVER", ignore = true },
]
payees = [{ name = "Acme", one_of = ["ACME HELP", "ACME SHOP"] }]
"#;
    let mut patterns = Patterns::embedded();
    patterns.add("rules.toml", rules).unwrap();
    let rows = read("joint.csv", csv).unwrap();
    let found = estimate(&spending_plan(GROCERIES), &rows, &patterns);
    assert_eq!(
        proposal_amount(&found, Category::Groceries),
        Some(600),
        "the rule over the bank's dining"
    );
    assert_eq!(proposal_amount(&found, Category::Housing), Some(18_000));
    let unknown: Vec<(&str, i64)> = (found.unknown.iter())
        .map(|it| (it.payee.as_str(), it.amount))
        .collect();
    assert_eq!(
        unknown,
        [("CHECK PAID", 480), ("REFUNDED THING", 372), ("Acme", 360)]
    );
    assert_eq!((found.ignored, found.ignored_rows), (420, 2));
    let amounts = |of: Of| -> Vec<i64> {
        records(&rows, &patterns, &of)
            .iter()
            .map(|row| row.amount.0)
            .collect()
    };
    assert_eq!(amounts(Of::Ignored), [-500, -3000]);
    assert_eq!(amounts(Of::Payee("Acme".to_owned())), [-2000, -1000]);
    let elsewhere = read("card.csv", csv).unwrap();
    let found = estimate(&spending_plan(GROCERIES), &elsewhere, &patterns);
    assert_eq!(
        proposal_amount(&found, Category::Housing),
        None,
        "another export's check"
    );
}

#[test]
fn a_payee_sent_to_the_bucket_lands_there_whatever_the_bank_says() {
    let csv = "Date,Description,Category,Amount\n2026-10-01,ART STUDIO,Home,-110.00\n2026-10-31,SAFEWAY,Groceries,-50.00\n";
    let mut patterns = Patterns::embedded();
    patterns
        .add(
            "rules.toml",
            "[[patterns]]\nis = \"ART STUDIO\"\nbucket = true\n",
        )
        .unwrap();
    let found = estimate(
        &spending_plan(GROCERIES),
        &read("x.csv", csv).unwrap(),
        &patterns,
    );
    let bucket = found.proposals.iter().find(|p| p.is_bucket).unwrap();
    assert_eq!(bucket.unknown, 1320);
    assert_eq!(found.unknown[0].payee, "ART STUDIO");
    assert!(
        Patterns::default()
            .add(
                "rules.toml",
                "[[patterns]]\nis = \"X\"\nbucket = true\nignore = true\n"
            )
            .is_err()
    );
}

/// Rows each sorted a different way, under the user's file and a proposed
/// one read after it.
fn sorting() -> (Vec<Row>, Patterns) {
    let csv = "Date,Description,Category,Type,Amount\n\
        2026-10-01,SAFEWAY #12,Groceries,Sale,-50.00\n\
        2026-10-02,ATT WIRELESS,,Sale,-80.00\n\
        2026-10-03,ACME HELP,,Payment,-20.00\n\
        2026-10-04,ART STUDIO,,Sale,-10.00\n\
        2026-10-05,QUICK FUEL 9,,Sale,-30.00\n\
        2026-10-31,SAFEWAY.COM ORDER,,Sale,-40.00\n";
    let mut patterns = Patterns::embedded();
    let mine = "patterns = [\n  { is = \"ART STUDIO\", bucket = true },\n  { match = \"FUEL\", category = \"fuel\" },\n]\n";
    patterns.add("patterns.toml", mine).unwrap();
    let proposed = "payees = [{ name = \"SAFEWAY\", match = \"SAFEWAY\" }]\n\
        patterns = [{ match = \"FUEL\", category = \"groceries\" }]\n";
    patterns.add("proposed.toml", proposed).unwrap();
    (read("x.csv", csv).unwrap(), patterns)
}

#[test]
fn each_record_says_what_sorted_it_and_a_later_file_wins_a_tie() {
    let (rows, patterns) = sorting();
    let decided = |payee: &str| {
        let found = sorted_records(&rows, &patterns, &Of::Containing(payee.to_owned()));
        assert_eq!(found.len(), 1, "{payee}");
        found[0].decided.clone()
    };
    let category = |category, by| Some((Outcome::Category(category), by));
    let rule = |file: &str, at| By::Rule {
        file: file.to_owned(),
        at,
    };
    assert_eq!(
        decided("safeway #12"),
        category(Category::Groceries, By::Bank)
    );
    assert_eq!(
        decided("att"),
        category(Category::Phone, By::Embedded("ATT ".to_owned()))
    );
    assert_eq!(decided("acme"), category(Category::Transfer, By::Type));
    assert_eq!(
        decided("art studio"),
        Some((Outcome::Bucket, rule("patterns.toml", 1)))
    );
    assert_eq!(
        decided("fuel"),
        category(Category::Groceries, rule("proposed.toml", 1)),
        "the later file wins the tie"
    );
}

#[test]
fn payees_containing_some_words_are_found_by_either_name_wherever_they_went() {
    let (rows, patterns) = sorting();
    let every = |of: Of| sorted_records(&rows, &patterns, &of);
    let safeway = every(Of::Containing("Safeway".to_owned()));
    assert_eq!(safeway.len(), 2, "by the merged name");
    assert_eq!(safeway[0].name, "SAFEWAY");
    assert_eq!(
        every(Of::Containing(".com".to_owned())).len(),
        1,
        "by the text as the bank wrote it"
    );
    assert_eq!(
        records(&rows, &patterns, &Of::Unnamed).len(),
        1,
        "the bucketed are counted unnamed"
    );
    let groceries = every(Of::Category(Category::Groceries));
    assert_eq!(groceries.len(), 3);
    assert_eq!(spent_a_year(&spending_plan(""), &rows, &groceries), 1440);
}
