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
            "gains_rate": "15",
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
    assert!(refused.contains("have no 99% bracket"), "{refused}");
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

#[test]
fn optimize_order_ranks_the_orders_and_stores_the_best() {
    let root = scratch_dir(
        "mcp-order",
        "plan.toml",
        &[("opt.toml", OPT_PLAN), ("claims.toml", CLAIMS_PLAN)],
    );
    let mut client = McpClient::spawn(&root);
    let reply = client.call(
        "optimize_order",
        json!({"path": "opt.toml", "write_to": "nested/order.toml"}),
    );
    let candidates = reply["candidates"].as_array().unwrap();
    assert!(candidates.len() >= 2, "{reply}");
    assert_eq!(
        candidates[0]["order"],
        json!(["taxable", "deferred", "roth", "hsa"]),
        "{reply}"
    );
    assert_eq!(
        candidates[0]["summary"], reply["baseline"],
        "the plan's own order is best here: {reply}"
    );
    assert_eq!(reply["written"], true, "{reply}");
    let scenario = reply["scenario_toml"].as_str().unwrap();
    assert!(scenario.contains("base = \"../opt.toml\""), "{scenario}");
    assert!(
        scenario.contains("[plan]\nwithdrawal_order = ["),
        "{scenario}"
    );
    let valid = client.call("validate_plan", json!({"path": "nested/order.toml"}));
    assert_eq!(valid["issues"].as_array().unwrap().len(), 0, "{valid}");

    let refused = client.call_expecting_error("optimize_order", json!({"path": "claims.toml"}));
    assert!(refused.contains("nothing to order"), "{refused}");
}

#[test]
fn optimize_spending_finds_both_ceilings_and_stores_the_one_at_the_target() {
    let few = format!("{OPT_PLAN}\n[market.monte_carlo]\ntrials = 100\n");
    let fixed = few.replace("amount = 40000", "amount = 40000\nessential = true");
    let root = scratch_dir(
        "mcp-spending",
        "plan.toml",
        &[("opt.toml", &few), ("fixed.toml", &fixed)],
    );
    let mut client = McpClient::spawn(&root);
    let reply = client.call(
        "optimize_spending",
        json!({"path": "opt.toml", "success": 0.8, "write_to": "nested/ceiling.toml"}),
    );
    assert_eq!(reply["flexible"], 40_000, "{reply}");
    let at_target = &reply["at_target"];
    assert!(at_target["success"].as_f64().unwrap() >= 0.8, "{reply}");
    assert!(
        at_target["flexible"].as_i64() < reply["planned"]["flexible"].as_i64(),
        "{reply}"
    );
    assert_eq!(reply["written"], true, "{reply}");
    let scenario = reply["scenario_toml"].as_str().unwrap();
    assert_eq!(
        scenario,
        format!(
            "schema = 1\nbase = \"../opt.toml\"\n\n[[expenses]]\nid = \"living\"\namount = {}\n",
            at_target["flexible"]
        )
    );
    let valid = client.call("validate_plan", json!({"path": "nested/ceiling.toml"}));
    assert_eq!(valid["issues"].as_array().unwrap().len(), 0, "{valid}");
    let runs = client.call("plan_monte_carlo", json!({"path": "nested/ceiling.toml"}));
    assert_eq!(runs["success_rate"], at_target["success"], "{runs}");

    let at_default = client.call("optimize_spending", json!({"path": "opt.toml"}));
    assert!(
        at_default["at_target"]["success"].as_f64().unwrap() >= 0.9,
        "{at_default}"
    );
    assert!(
        at_default["at_target"]["flexible"].as_i64() < at_target["flexible"].as_i64(),
        "nine markets in ten leave less to spend than eight: {at_default}"
    );
    assert_eq!(at_default["written"], false, "{at_default}");

    let refused = client.call_expecting_error("optimize_spending", json!({"path": "fixed.toml"}));
    assert!(refused.contains("no flexible spending"), "{refused}");
}
