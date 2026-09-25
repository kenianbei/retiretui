use std::collections::BTreeMap;

use super::triggers::TriggerForm;
use super::validate::{Issue, push_issue};
use super::{
    Allocation, Cliff, Contribution, Conversion, Expense, Income, Node, Plan, Span, Trigger,
};

const MAX_TRIGGER_AGE: u8 = 120;

pub(crate) fn check_references(plan: &Plan, issues: &mut Vec<Issue>) {
    for (path, trigger) in collect_triggers(plan) {
        match trigger.form() {
            Err(message) => issues.push(Issue { path, message }),
            Ok(form) => check_trigger_refs(plan, &path, form, issues),
        }
    }
    check_cycles(plan, issues);
}

fn check_trigger_refs(plan: &Plan, path: &str, form: TriggerForm<'_>, issues: &mut Vec<Issue>) {
    match form {
        TriggerForm::Date(_) => {}
        TriggerForm::Age { owner, years } => {
            if plan.person(owner).is_none() {
                push_issue(issues, path, format!("unknown person `{owner}`"));
            }
            if years > MAX_TRIGGER_AGE {
                push_issue(issues, path, format!("age above {MAX_TRIGGER_AGE}"));
            }
        }
        TriggerForm::Event { id, .. } => {
            if !plan.events.iter().any(|event| event.id == id) {
                push_issue(issues, path, format!("unknown event `{id}`"));
            }
        }
        TriggerForm::Income { id, .. } => {
            if plan.income_source(id).is_none() {
                push_issue(issues, path, format!("unknown income `{id}`"));
            }
        }
    }
}

fn check_cycles(plan: &Plan, issues: &mut Vec<Issue>) {
    let mut marks = BTreeMap::new();
    let nodes: Vec<Node<'_>> = plan
        .events
        .iter()
        .map(|event| Node::Event(&event.id))
        .chain(plan.income.iter().map(|income| Node::Income(&income.id)))
        .collect();
    for node in nodes {
        visit(plan, node, &mut marks, issues);
    }
}

impl Node<'_> {
    fn path(self) -> String {
        match self {
            Self::Event(id) => format!("events.{id}"),
            Self::Income(id) => format!("income.{id}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Visiting,
    Done,
}

fn node_dependency<'a>(plan: &'a Plan, node: Node<'a>) -> Option<Node<'a>> {
    let trigger: Option<&Trigger> = match node {
        Node::Event(id) => plan
            .events
            .iter()
            .find(|event| event.id == id)
            .map(|event| &event.trigger),
        Node::Income(id) => plan
            .income_source(id)
            .and_then(|income| income.span().first()),
    };
    match trigger?.form().ok()? {
        TriggerForm::Event { id, .. } => Some(Node::Event(id)),
        TriggerForm::Income { id, .. } => Some(Node::Income(id)),
        TriggerForm::Date(_) | TriggerForm::Age { .. } => None,
    }
}

fn visit<'a>(
    plan: &'a Plan,
    node: Node<'a>,
    marks: &mut BTreeMap<Node<'a>, Mark>,
    issues: &mut Vec<Issue>,
) -> bool {
    match marks.get(&node) {
        Some(Mark::Done) => return false,
        Some(Mark::Visiting) => {
            issues.push(Issue {
                path: node.path(),
                message: "part of a trigger reference cycle".into(),
            });
            return true;
        }
        None => {}
    }
    marks.insert(node, Mark::Visiting);
    let cycled = node_dependency(plan, node).is_some_and(|dep| visit(plan, dep, marks, issues));
    marks.insert(node, Mark::Done);
    cycled
}

fn collect_triggers(plan: &Plan) -> Vec<(String, &Trigger)> {
    let mut all: Vec<(String, &Trigger)> = Vec::new();
    for (i, event) in plan.events.iter().enumerate() {
        all.push((format!("events[{i}].trigger"), &event.trigger));
    }
    collect_account_triggers(plan, &mut all);
    collect_window_triggers(plan, &mut all);
    collect_flow_triggers(plan, &mut all);
    all
}

fn collect_account_triggers<'a>(plan: &'a Plan, all: &mut Vec<(String, &'a Trigger)>) {
    for (i, account) in plan.accounts.iter().enumerate() {
        if let Some(trigger) = &account.locked_until {
            all.push((format!("accounts[{i}].locked_until"), trigger));
        }
        if let Some(Allocation::GlidePath(phases)) = &account.allocation {
            for (at, phase) in phases.iter().enumerate() {
                all.push((format!("accounts[{i}].allocation[{at}].from"), &phase.from));
            }
        }
    }
}

fn collect_window_triggers<'a>(plan: &'a Plan, all: &mut Vec<(String, &'a Trigger)>) {
    all.extend(span_triggers(
        "income",
        plan.income.iter().map(Income::span),
    ));
    all.extend(span_triggers(
        "expenses",
        plan.expenses.iter().map(Expense::span),
    ));
    all.extend(span_triggers("cliffs", plan.cliffs.iter().map(Cliff::span)));
}

fn collect_flow_triggers<'a>(plan: &'a Plan, all: &mut Vec<(String, &'a Trigger)>) {
    for (i, transfer) in plan.transfers.iter().enumerate() {
        all.push((format!("transfers[{i}].on"), &transfer.on));
    }
    all.extend(span_triggers(
        "conversions",
        plan.conversions.iter().map(Conversion::span),
    ));
    all.extend(span_triggers(
        "contributions",
        plan.contributions.iter().map(Contribution::span),
    ));
    for (i, residency) in plan.residency.iter().enumerate() {
        if let Some(trigger) = &residency.from {
            all.push((format!("residency[{i}].from"), trigger));
        }
    }
}

fn span_triggers<'a>(
    section: &'static str,
    spans: impl Iterator<Item = Span<'a>>,
) -> impl Iterator<Item = (String, &'a Trigger)> {
    spans.enumerate().flat_map(move |(i, span)| {
        span.triggers()
            .map(move |(key, trigger)| (format!("{section}[{i}].{key}"), trigger))
    })
}
