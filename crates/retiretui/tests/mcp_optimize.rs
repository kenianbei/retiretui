//! MCP optimizer tool tests, speaking JSON-RPC to the built binary over
//! stdio.

mod common;

use serde_json::json;

use common::mcp::McpClient;
use common::{CLAIMS_PLAN, OPT_PLAN, scratch_dir};

#[test]
fn optimizer_tools_sweep_emit_and_store() {
    let root = scratch_dir("mcp-optimize", "plan.toml", &[("opt.toml", OPT_PLAN)]);
    let mut client = McpClient::spawn(&root);
    let sweep = client.call(
        "sweep_conversion_brackets",
        json!({"path": "opt.toml", "from": ["k"], "to": "r"}),
    );
    let brackets = sweep["brackets"].as_array().unwrap();
    assert!(brackets.len() >= 2, "{sweep}");
    assert!(sweep["baseline"]["final_net_worth"].is_i64(), "{sweep}");

    let ladder = client.call(
        "optimize_conversions",
        json!({
            "path": "opt.toml",
            "from": ["k"],
            "to": "r",
            "bracket": 12,
            "max_magi": 500_000,
            "write_to": "nested/ladder.toml",
        }),
    );
    assert!(!ladder["steps"].as_array().unwrap().is_empty(), "{ladder}");
    assert_eq!(ladder["written"], true, "{ladder}");
    let scenario = ladder["scenario_toml"].as_str().unwrap();
    assert!(scenario.contains("base = \"../opt.toml\""), "{scenario}");
    assert!(scenario.contains("\ndate = 2026-01-01\n"), "{scenario}");
    let stored = std::fs::read_to_string(root.join("nested/ladder.toml")).unwrap();
    assert!(stored.contains("\ndate = 2026-01-01\n"), "{stored}");
    let valid = client.call("validate_plan", json!({"path": "nested/ladder.toml"}));
    assert_eq!(valid["issues"].as_array().unwrap().len(), 0, "{valid}");
    let compared = client.call(
        "compare_plans",
        json!({"paths": ["opt.toml", "nested/ladder.toml"]}),
    );
    assert_eq!(compared["plans"].as_array().unwrap().len(), 2, "{compared}");

    let refused = client.call_expecting_error(
        "optimize_conversions",
        json!({"path": "opt.toml", "from": ["k"], "to": "r", "bracket": 99}),
    );
    assert!(refused.contains("no bracket"), "{refused}");
    let needs_bracket = client.call_expecting_error(
        "optimize_conversions",
        json!({"path": "opt.toml", "from": ["k"], "to": "r"}),
    );
    assert!(needs_bracket.contains("bracket"), "{needs_bracket}");
}

#[test]
fn optimize_claims_ranks_the_grid_and_stores_the_best() {
    let root = scratch_dir("mcp-claims", "plan.toml", &[("claims.toml", CLAIMS_PLAN)]);
    let mut client = McpClient::spawn(&root);
    let reply = client.call(
        "optimize_claims",
        json!({"path": "claims.toml", "write_to": "nested/claims.toml"}),
    );
    assert_eq!(reply["incomes"], json!(["ss"]), "{reply}");
    let candidates = reply["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 9, "{reply}");
    assert_eq!(candidates[0]["claims"][0]["income"], "ss", "{reply}");
    assert!(reply["baseline"]["final_net_worth"].is_i64(), "{reply}");
    assert_eq!(reply["written"], true, "{reply}");
    let scenario = reply["scenario_toml"].as_str().unwrap();
    assert!(scenario.contains("base = \"../claims.toml\""), "{scenario}");
    assert!(scenario.contains("[income.start]\nage = "), "{scenario}");
    let valid = client.call("validate_plan", json!({"path": "nested/claims.toml"}));
    assert_eq!(valid["issues"].as_array().unwrap().len(), 0, "{valid}");

    let refused = client.call_expecting_error(
        "optimize_claims",
        json!({"path": "claims.toml", "incomes": ["nope"]}),
    );
    assert!(refused.contains("unknown income"), "{refused}");
}
