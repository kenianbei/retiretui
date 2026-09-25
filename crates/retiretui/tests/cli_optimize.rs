//! `optimize` CLI behavior tests, driving the built binary.

mod common;

use std::process::Output;

use common::{CLAIMS_PLAN, OPT_PLAN, json_of, retiretui, scratch_dir};

/// `optimize conversions` on `plan` from `k` into `r`, plus `extra`.
fn conversions(plan: &str, extra: &[&str]) -> Output {
    let mut args = vec!["optimize", "conversions", plan, "--from", "k", "--to", "r"];
    args.extend_from_slice(extra);
    retiretui(&args)
}

#[test]
fn optimize_sweeps_and_writes_a_comparable_ladder() {
    let dir = scratch_dir("cli-optimize", "base.toml", &[("opt.toml", OPT_PLAN)]);
    let plan = dir.join("opt.toml");
    let plan = plan.to_str().unwrap();
    let sweep = conversions(plan, &[]);
    assert!(sweep.status.success(), "{sweep:?}");
    let stdout = String::from_utf8(sweep.stdout).unwrap();
    assert!(stdout.contains("baseline"), "{stdout}");
    assert!(stdout.contains("12%"), "{stdout}");
    assert!(stdout.contains("22%"), "{stdout}");

    let ladder = dir.join("ladder.toml");
    let single = conversions(
        plan,
        &["--bracket", "12", "--write", ladder.to_str().unwrap()],
    );
    assert!(single.status.success(), "{single:?}");
    let stdout = String::from_utf8(single.stdout).unwrap();
    assert!(stdout.contains("optimized"), "{stdout}");
    assert!(stdout.contains("2026"), "{stdout}");

    let capped = conversions(
        plan,
        &[
            "--bracket",
            "12",
            "--max-magi",
            "30000",
            "--start-year",
            "2041",
        ],
    );
    assert!(capped.status.success(), "{capped:?}");

    let written = std::fs::read_to_string(&ladder).unwrap();
    assert!(written.contains("\ndate = 2026-01-01\n"), "{written}");
    let ladder = ladder.to_str().unwrap();
    let valid = retiretui(&["validate", ladder]);
    assert!(valid.status.success(), "{valid:?}");
    let compared = retiretui(&["compare", plan, ladder]);
    assert!(compared.status.success(), "{compared:?}");
    let stdout = String::from_utf8(compared.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 3, "{stdout}");
}

#[test]
fn optimize_claims_ranks_the_grid_and_writes_the_best() {
    let dir = scratch_dir("cli-claims", "base.toml", &[("claims.toml", CLAIMS_PLAN)]);
    let plan = dir.join("claims.toml");
    let plan = plan.to_str().unwrap();
    let ranked = retiretui(&["optimize", "claims", plan]);
    assert!(ranked.status.success(), "{ranked:?}");
    let stdout = String::from_utf8(ranked.stdout).unwrap();
    let header = stdout.lines().next().unwrap();
    assert!(header.trim_start().starts_with("rank"), "{stdout}");
    assert!(header.contains(" ss "), "{stdout}");
    assert!(
        stdout.lines().nth(1).unwrap().starts_with("baseline"),
        "{stdout}"
    );
    assert_eq!(
        stdout.lines().count(),
        11,
        "a header, the baseline, nine ages: {stdout}"
    );

    let overlay = dir.join("best.toml");
    let written = retiretui(&[
        "optimize",
        "claims",
        plan,
        "--write",
        overlay.to_str().unwrap(),
    ]);
    assert!(written.status.success(), "{written:?}");
    let text = std::fs::read_to_string(&overlay).unwrap();
    assert!(text.contains("[[income]]\nid = \"ss\"\n"), "{text}");
    let overlay = overlay.to_str().unwrap();
    let valid = retiretui(&["validate", overlay]);
    assert!(valid.status.success(), "{valid:?}");
    let compared = retiretui(&["compare", plan, overlay]);
    assert!(compared.status.success(), "{compared:?}");

    let refused = retiretui(&["optimize", "claims", plan, "--income", "nope"]);
    assert!(!refused.status.success());
    let stderr = String::from_utf8(refused.stderr).unwrap();
    assert!(stderr.contains("unknown income"), "{stderr}");
}

#[test]
fn optimize_claims_adds_the_benefit_a_record_lacks() {
    let unclaimed = CLAIMS_PLAN.replace(
        "[[income]]\nid = \"ss\"\nkind = \"social-security\"\nowner = \"me\"\nstart = { age = 67, owner = \"me\" }\n",
        "",
    );
    assert!(!unclaimed.contains("social-security"));
    let dir = scratch_dir(
        "cli-claims-added",
        "base.toml",
        &[("claims.toml", &unclaimed)],
    );
    let plan = dir.join("claims.toml");
    let overlay = dir.join("best.toml");
    let written = retiretui(&[
        "optimize",
        "claims",
        plan.to_str().unwrap(),
        "--write",
        overlay.to_str().unwrap(),
    ]);
    assert!(written.status.success(), "{written:?}");
    let text = std::fs::read_to_string(&overlay).unwrap();
    assert!(
        text.contains("id = \"ss-me\"\nkind = \"social-security\"\nowner = \"me\""),
        "{text}"
    );
    let valid = retiretui(&["validate", overlay.to_str().unwrap()]);
    assert!(valid.status.success(), "{valid:?}");
}

#[test]
fn optimize_write_requires_a_bracket() {
    let dir = scratch_dir("cli-optimize-req", "base.toml", &[("opt.toml", OPT_PLAN)]);
    let plan = dir.join("opt.toml");
    let refused = conversions(
        plan.to_str().unwrap(),
        &["--write", dir.join("x.toml").to_str().unwrap()],
    );
    assert!(!refused.status.success());
}

/// The names of the fields a JSON object holds, sorted.
fn fields(value: &serde_json::Value) -> Vec<&str> {
    let mut names: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    names.sort_unstable();
    names
}

#[test]
fn optimize_json_replies_name_their_fields() {
    let dir = scratch_dir(
        "cli-optimize-json",
        "base.toml",
        &[("opt.toml", OPT_PLAN), ("claims.toml", CLAIMS_PLAN)],
    );
    let opt = dir.join("opt.toml");
    let opt = opt.to_str().unwrap();
    let summary = [
        "final_deferred",
        "final_net_worth",
        "first_unfunded_year",
        "lifetime_conversions",
        "lifetime_medicare",
        "lifetime_taxes",
        "lifetime_unfunded",
        "peak_net_worth",
        "peak_year",
    ];

    let sweep = json_of(&conversions(opt, &["--format", "json"]));
    assert_eq!(fields(&sweep), ["baseline", "brackets"]);
    assert_eq!(fields(&sweep["baseline"]), summary);
    let bracket = &sweep["brackets"][0];
    assert_eq!(
        fields(bracket),
        ["bracket_rate", "optimized", "total_converted"]
    );
    assert_eq!(fields(&bracket["optimized"]), summary);

    let ladder = json_of(&conversions(opt, &["--bracket", "12", "--format", "json"]));
    assert_eq!(
        fields(&ladder),
        ["baseline", "optimized", "steps", "total_converted"]
    );
    assert_eq!(fields(&ladder["steps"][0]), ["amount", "source", "year"]);
    assert_eq!(fields(&ladder["optimized"]), summary);

    let claims = dir.join("claims.toml");
    let claims = json_of(&retiretui(&[
        "optimize",
        "claims",
        claims.to_str().unwrap(),
        "--format",
        "json",
    ]));
    assert_eq!(fields(&claims), ["baseline", "candidates", "incomes"]);
    let candidate = &claims["candidates"][0];
    assert_eq!(fields(candidate), ["claims", "summary"]);
    assert_eq!(fields(&candidate["claims"][0]), ["age", "income", "owner"]);
    assert_eq!(fields(&candidate["summary"]), summary);
}
