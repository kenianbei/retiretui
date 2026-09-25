//! MCP server behavior tests, speaking JSON-RPC to the built binary over
//! stdio.

mod common;

use serde_json::json;

use common::mcp::McpClient;
use common::{FULL_PLAN, STATEMENT, scratch_dir};

#[test]
fn lists_tools_and_finds_plans_recursively() {
    let root = scratch_dir("mcp-list", "plan.toml", &[]);
    std::fs::copy(FULL_PLAN, root.join("nested/other.toml")).unwrap();
    let mut client = McpClient::spawn(&root);
    let tools = client.request("tools/list", json!({}));
    let names: Vec<&str> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    for expected in [
        "list_plans",
        "read_plan",
        "validate_plan",
        "write_plan",
        "project_plan",
        "compare_plans",
        "optimize_conversions",
        "sweep_conversion_brackets",
        "optimize_claims",
        "plan_actions",
        "plan_monte_carlo",
        "plan_historical",
        "tax_parameters",
        "describe_schema",
    ] {
        assert!(names.contains(&expected), "missing {expected} in {names:?}");
    }
    let plans = client.call("list_plans", json!({}));
    let paths: Vec<&str> = plans["plans"]
        .as_array()
        .unwrap()
        .iter()
        .map(|plan| plan["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"plan.toml"), "{paths:?}");
    assert!(paths.contains(&"nested/other.toml"), "{paths:?}");
}

#[test]
fn reads_and_validates_a_plan() {
    let root = scratch_dir("mcp-read", "plan.toml", &[]);
    let mut client = McpClient::spawn(&root);
    let read = client.call("read_plan", json!({"path": "plan.toml"}));
    assert!(read["text"].as_str().unwrap().contains("schema = 1"));
    let valid = client.call("validate_plan", json!({"path": "plan.toml"}));
    assert_eq!(valid["issues"].as_array().unwrap().len(), 0, "{valid}");
}

#[test]
fn plan_actions_reports_a_year() {
    let root = scratch_dir("mcp-actions", "plan.toml", &[]);
    let mut client = McpClient::spawn(&root);
    let report = client.call("plan_actions", json!({"path": "plan.toml", "year": 2040}));
    assert_eq!(report["year"], 2040);
    let actions = report["actions"].as_array().unwrap();
    assert!(!actions.is_empty(), "{report}");
    assert!(actions.iter().all(|action| action["kind"].is_string()));
    let refused =
        client.call_expecting_error("plan_actions", json!({"path": "plan.toml", "year": 1990}));
    assert!(refused.contains("2026-2074"), "{refused}");
}

#[test]
fn write_plan_round_trips_a_valid_document() {
    let root = scratch_dir("mcp-write", "plan.toml", &[]);
    let submitted = std::fs::read_to_string(FULL_PLAN).unwrap();
    let mut client = McpClient::spawn(&root);
    let written = client.call(
        "write_plan",
        json!({"path": "nested/new.toml", "toml": submitted}),
    );
    assert_eq!(written["issues"].as_array().unwrap().len(), 0, "{written}");
    let read = client.call("read_plan", json!({"path": "nested/new.toml"}));
    let stored = read["text"].as_str().unwrap();
    assert!(stored.contains("schema = 1"));
    // The fixture has comments, so the canonical form must differ.
    assert_eq!(written["canonicalized"], true, "{written}");
    assert!(!stored.contains('#'));
}

#[test]
fn write_plan_refuses_an_invalid_document_and_writes_nothing() {
    let root = scratch_dir("mcp-write-invalid", "plan.toml", &[]);
    let fixture = std::fs::read_to_string(FULL_PLAN).unwrap();
    let invalid = fixture.replace("inflation = 0.025", "inflation = 9.0");
    assert_ne!(invalid, fixture);
    let mut client = McpClient::spawn(&root);
    let refusal = client.call("write_plan", json!({"path": "bad.toml", "toml": invalid}));
    assert!(
        !refusal["issues"].as_array().unwrap().is_empty(),
        "{refusal}"
    );
    assert!(!root.join("bad.toml").exists());
    let garbage = client.call_expecting_error(
        "write_plan",
        json!({"path": "bad.toml", "toml": "not toml ["}),
    );
    assert!(garbage.contains("bad.toml"), "{garbage}");
    let extension =
        client.call_expecting_error("write_plan", json!({"path": "plan.json", "toml": fixture}));
    assert!(extension.contains(".toml"), "{extension}");
}

#[test]
fn project_plan_summarizes_trims_and_expands() {
    let root = scratch_dir("mcp-project", "plan.toml", &[]);
    let mut client = McpClient::spawn(&root);
    let summary = client.call(
        "project_plan",
        json!({"path": "plan.toml", "from_year": 2027, "to_year": 2029}),
    );
    let years = summary["years"].as_array().unwrap();
    assert_eq!(years.len(), 3, "{summary}");
    assert_eq!(years[0]["year"], 2027);
    for field in ["total_income", "class_totals", "net_worth", "deflator"] {
        assert!(years[0].get(field).is_some(), "missing {field}");
    }
    assert!(
        years[0].get("balances").is_none(),
        "summary carries accounts"
    );
    let full = client.call(
        "project_plan",
        json!({"path": "plan.toml", "to_year": 2026, "detail": "full"}),
    );
    let first = &full["years"].as_array().unwrap()[0];
    assert!(first.get("balances").is_some(), "{first}");
    assert!(first.get("income").is_some(), "{first}");
}

#[test]
fn project_plan_refuses_an_invalid_plan() {
    let root = scratch_dir("mcp-project-invalid", "plan.toml", &[]);
    let broken = std::fs::read_to_string(root.join("plan.toml"))
        .unwrap()
        .replace("inflation = 0.025", "inflation = 9.0");
    std::fs::write(root.join("plan.toml"), broken).unwrap();
    let mut client = McpClient::spawn(&root);
    let refusal = client.call_expecting_error("project_plan", json!({"path": "plan.toml"}));
    assert!(refusal.contains("invalid"), "{refusal}");
    assert!(refusal.contains("inflation"), "{refusal}");
}

#[test]
fn tax_parameters_resolve_and_extension_needs_inflation() {
    let root = scratch_dir("mcp-tax", "plan.toml", &[]);
    let mut client = McpClient::spawn(&root);
    let known = client.call("tax_parameters", json!({"year": 2026}));
    assert_eq!(known["deductions"]["standard"]["single"], 16100, "{known}");
    let refusal = client.call_expecting_error("tax_parameters", json!({"year": 2060}));
    assert!(refusal.contains("inflation"), "{refusal}");
    let extended = client.call("tax_parameters", json!({"year": 2060, "inflation": 0.02}));
    assert!(
        extended["deductions"]["standard"]["single"]
            .as_i64()
            .unwrap()
            > 16100,
        "{extended}"
    );
}

#[test]
fn describe_schema_example_passes_write_plan() {
    let root = scratch_dir("mcp-schema", "plan.toml", &[]);
    let mut client = McpClient::spawn(&root);
    let reply = client.call_raw("describe_schema", json!({}));
    let text = reply["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("## Triggers"), "schema reference truncated");
    let example = text
        .split("## Worked example")
        .nth(1)
        .and_then(|tail| tail.split("```toml\n").nth(1))
        .and_then(|block| block.split("```").next())
        .expect("worked example block");
    let written = client.call(
        "write_plan",
        json!({"path": "example.toml", "toml": example}),
    );
    assert_eq!(written["issues"].as_array().unwrap().len(), 0, "{written}");
}

#[test]
fn schema_reference_names_the_full_vocabulary() {
    let text = include_str!("../src/commands/mcp/schema.md");
    let account_kinds = [
        "401k",
        "403b",
        "457b",
        "414k",
        "ira",
        "sep-ira",
        "simple-ira",
        "hsa",
        "brokerage",
        "cash",
    ];
    for kind in account_kinds {
        assert!(
            text.contains(&format!("`\"{kind}\"`")),
            "missing account kind {kind}"
        );
    }
    let income_kinds = [
        "salary",
        "pension",
        "annuity",
        "rental",
        "social-security",
        "windfall",
        "other",
    ];
    for kind in income_kinds {
        assert!(
            text.contains(&format!("`\"{kind}\"`")),
            "missing income kind {kind}"
        );
    }
    for basis in ["date =", "age =", "event =", "income ="] {
        assert!(text.contains(basis), "missing trigger basis {basis}");
    }
    for marker in ["base =", "remove = true", "replace = true"] {
        assert!(text.contains(marker), "missing scenario marker {marker}");
    }
    for marker in ["[medicare]", "[[cliffs]]", "`magi_over`", "`prior_magi`"] {
        assert!(text.contains(marker), "missing cliff marker {marker}");
    }
    for marker in ["[[contributions]]", "`\"employer\"`"] {
        assert!(
            text.contains(marker),
            "missing contribution marker {marker}"
        );
    }
}

const SCENARIO: &str = "schema = 1\nbase = \"plan.toml\"\n\n[plan]\nname = \"retire-early\"\n\n[[expenses]]\nid = \"travel\"\nremove = true\n";

#[test]
fn scenarios_resolve_across_tools() {
    let root = scratch_dir("mcp-scenario", "plan.toml", &[("early.toml", SCENARIO)]);
    let mut client = McpClient::spawn(&root);
    let valid = client.call("validate_plan", json!({"path": "early.toml"}));
    assert_eq!(valid["issues"].as_array().unwrap().len(), 0, "{valid}");
    let projected = client.call(
        "project_plan",
        json!({"path": "early.toml", "to_year": 2026}),
    );
    assert_eq!(
        projected["years"].as_array().unwrap().len(),
        1,
        "{projected}"
    );
    let plans = client.call("list_plans", json!({}));
    let entries = plans["plans"].as_array().unwrap();
    let early = entries.iter().find(|e| e["path"] == "early.toml").unwrap();
    assert_eq!(early["base"], "plan.toml", "{early}");
    assert_eq!(early["name"], "retire-early", "{early}");
    let base = entries.iter().find(|e| e["path"] == "plan.toml").unwrap();
    assert!(base["base"].is_null(), "{base}");
}

#[test]
fn write_plan_accepts_scenario_documents() {
    let root = scratch_dir("mcp-write-scenario", "plan.toml", &[]);
    let mut client = McpClient::spawn(&root);
    let nested = SCENARIO.replace("plan.toml", "../plan.toml");
    let written = client.call(
        "write_plan",
        json!({"path": "nested/early.toml", "toml": nested}),
    );
    assert_eq!(written["issues"].as_array().unwrap().len(), 0, "{written}");
    let read = client.call("read_plan", json!({"path": "nested/early.toml"}));
    let stored = read["text"].as_str().unwrap();
    assert!(stored.contains("base = \"../plan.toml\""), "{stored}");
    let unmatched = SCENARIO.replace("id = \"travel\"", "id = \"ghost\"");
    let refusal =
        client.call_expecting_error("write_plan", json!({"path": "bad.toml", "toml": unmatched}));
    assert!(refusal.contains("matched no"), "{refusal}");
    assert!(!root.join("bad.toml").exists());
    let invalid = SCENARIO.replace(
        "name = \"retire-early\"",
        "name = \"retire-early\"\ninflation = 9.0",
    );
    let blocked = client.call("write_plan", json!({"path": "bad.toml", "toml": invalid}));
    assert!(
        !blocked["issues"].as_array().unwrap().is_empty(),
        "{blocked}"
    );
    assert!(!root.join("bad.toml").exists());
}

#[test]
fn compare_plans_summarizes_each_path() {
    let root = scratch_dir("mcp-compare", "plan.toml", &[("early.toml", SCENARIO)]);
    let mut client = McpClient::spawn(&root);
    let reply = client.call(
        "compare_plans",
        json!({"paths": ["plan.toml", "early.toml"]}),
    );
    let plans = reply["plans"].as_array().unwrap();
    assert_eq!(plans.len(), 2, "{reply}");
    assert_eq!(plans[1]["name"], "retire-early");
    assert!(plans[0]["summary"]["final_net_worth"].is_i64(), "{reply}");
    assert!(plans[0]["summary"]["peak_year"].is_i64(), "{reply}");
    let single = client.call_expecting_error("compare_plans", json!({"paths": ["plan.toml"]}));
    assert!(single.contains("two"), "{single}");
}

#[test]
fn rejects_paths_outside_the_root() {
    let root = scratch_dir("mcp-escape", "plan.toml", &[]);
    let mut client = McpClient::spawn(&root);
    let absolute = client.call_expecting_error("read_plan", json!({"path": "/etc/hostname"}));
    assert!(absolute.contains("absolute"), "{absolute}");
    let traversal = client.call_expecting_error("read_plan", json!({"path": "../outside.toml"}));
    assert!(traversal.contains("escapes"), "{traversal}");
}

#[test]
fn import_earnings_writes_the_record_through_the_store() {
    let root = scratch_dir("mcp-import-earnings", "plan.toml", &[]);
    let statement = std::fs::read_to_string(STATEMENT).unwrap();
    let mut client = McpClient::spawn(&root);
    let wrong = client.call_expecting_error(
        "import_earnings",
        json!({"path": "plan.toml", "person": "alex", "statement": statement}),
    );
    assert!(wrong.contains("born"), "{wrong}");
    let reply = client.call(
        "import_earnings",
        json!({"path": "plan.toml", "person": "jordan", "statement": statement}),
    );
    assert_eq!(reply["issues"].as_array().unwrap().len(), 0, "{reply}");
    let text = client.call("read_plan", json!({"path": "plan.toml"}));
    let text = text["text"].as_str().unwrap();
    assert!(text.contains("2024 = 168600"), "{text}");
}

#[test]
fn market_tools_run_the_plan_and_stay_inside_the_root() {
    let root = scratch_dir("mcp-markets", "plan.toml", &[]);
    std::fs::write(
        root.join("few.toml"),
        "schema = 1\nbase = \"plan.toml\"\n\n[market.monte_carlo]\ntrials = 30\n",
    )
    .unwrap();
    let mut client = McpClient::spawn(&root);
    let drawn = client.call("plan_monte_carlo", json!({"path": "few.toml"}));
    assert_eq!(drawn["runs"], 30);
    assert_eq!(drawn["markets"].as_array().unwrap().len(), 7);
    let history = client.call("plan_historical", json!({"path": "plan.toml"}));
    assert_eq!(history["runs"], 155);
    let escaped =
        client.call_expecting_error("plan_monte_carlo", json!({"path": "../outside.toml"}));
    assert!(escaped.contains("escapes"), "{escaped}");
}

#[test]
fn schema_reference_documents_the_market() {
    let text = include_str!("../src/commands/mcp/schema.md");
    for marker in [
        "[market]",
        "`allocation`",
        "`\"assumptions\"`",
        "`\"history\"`",
    ] {
        assert!(text.contains(marker), "missing market marker {marker}");
    }
}
