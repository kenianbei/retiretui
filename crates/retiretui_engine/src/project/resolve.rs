use std::collections::BTreeMap;

use crate::plan::{Allocation, Node, Plan, Span, Trigger, TriggerForm};

/// Trigger-to-year resolution, memoized over the plan's events and income
/// starts. Built once per projection; a validated plan always resolves, and
/// anything unresolvable simply never fires.
#[derive(Debug)]
pub(crate) struct Resolver {
    event_year: BTreeMap<String, Option<i16>>,
    income_year: BTreeMap<String, Option<i16>>,
}

impl Resolver {
    pub(crate) fn new(plan: &Plan) -> Self {
        let mut resolver = Self {
            event_year: BTreeMap::new(),
            income_year: BTreeMap::new(),
        };
        for event in &plan.events {
            let year = resolve_node(plan, Node::Event(&event.id), 0);
            resolver.event_year.insert(event.id.clone(), year);
        }
        for income in &plan.income {
            let year = resolve_node(plan, Node::Income(&income.id), 0);
            resolver.income_year.insert(income.id.clone(), year);
        }
        resolver
    }

    /// The calendar year a trigger fires, if it resolves.
    pub(crate) fn trigger_year(&self, plan: &Plan, trigger: &Trigger) -> Option<i16> {
        resolve_form(plan, trigger.form().ok()?, |node| match node {
            Node::Event(id) => self.event_year.get(id).copied().flatten(),
            Node::Income(id) => self.income_year.get(id).copied().flatten(),
        })
    }

    /// Whether an item with these triggers is active in `year`. A one-time
    /// `on` item is active only in its trigger's year; otherwise the window
    /// runs from `start` (default: plan start) through `end` (default: the
    /// horizon), and a trigger that fails to resolve deactivates the item
    /// rather than guessing.
    pub(crate) fn is_active(&self, plan: &Plan, year: i16, span: Span<'_>) -> bool {
        if let Some(on) = span.on {
            return self.trigger_year(plan, on) == Some(year);
        }
        let bound = |trigger: Option<&Trigger>, default: i16| match trigger {
            Some(trigger) => self.trigger_year(plan, trigger),
            None => Some(default),
        };
        let (Some(start), Some(end)) = (
            bound(span.start, plan.plan.start_year),
            bound(span.end, super::horizon_year(plan)),
        ) else {
            return false;
        };
        (start..=end).contains(&year)
    }
}

/// The calendar years a plan's dated items land in, as the projection
/// resolves them. `None` wherever the plan states no trigger, or states
/// one that never fires; a default window bound is not filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeline {
    /// Each event's year, by id.
    pub events: BTreeMap<String, Option<i16>>,
    /// Each income's window, by id.
    pub income: BTreeMap<String, Window>,
    /// Each contribution's window, by id.
    pub contributions: BTreeMap<String, Window>,
    /// Each residency's `from` year, in the plan's order.
    pub residency: Vec<Option<i16>>,
    /// Each glide-path step's `from` year, in the plan's order, by the id of
    /// the account holding it; accounts without a glide path are absent.
    pub glide_steps: BTreeMap<String, Vec<Option<i16>>>,
}

/// The years an item's `start`, `end` and `on` triggers land in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// The first year, where `start` is stated.
    pub start: Option<i16>,
    /// The last year, where `end` is stated.
    pub end: Option<i16>,
    /// The one year, where `on` is stated.
    pub on: Option<i16>,
}

impl Window {
    /// The item's first year where a trigger states one: `start`, else `on`.
    #[must_use]
    pub fn first(&self) -> Option<i16> {
        self.start.or(self.on)
    }
}

impl Timeline {
    /// Resolves every item's triggers in `plan`, which should have passed
    /// validation.
    #[must_use]
    pub fn new(plan: &Plan) -> Self {
        let resolver = Resolver::new(plan);
        let year = |trigger: Option<&Trigger>| resolver.trigger_year(plan, trigger?);
        let window = |span: Span<'_>| Window {
            start: year(span.start),
            end: year(span.end),
            on: year(span.on),
        };
        let glide_steps = plan.accounts.iter().filter_map(|account| {
            let Some(Allocation::GlidePath(steps)) = &account.allocation else {
                return None;
            };
            let years = steps.iter().map(|step| year(Some(&step.from)));
            Some((account.id.clone(), years.collect()))
        });
        Self {
            events: resolver.event_year.clone(),
            income: plan
                .income
                .iter()
                .map(|income| (income.id.clone(), window(income.span())))
                .collect(),
            contributions: plan
                .contributions
                .iter()
                .map(|contribution| (contribution.id.clone(), window(contribution.span())))
                .collect(),
            residency: plan
                .residency
                .iter()
                .map(|residency| year(residency.from.as_ref()))
                .collect(),
            glide_steps: glide_steps.collect(),
        }
    }
}

fn resolve_node(plan: &Plan, node: Node<'_>, depth: u32) -> Option<i16> {
    const MAX_DEPTH: u32 = 64;
    if depth > MAX_DEPTH {
        return None;
    }
    let trigger: &Trigger = match node {
        Node::Event(id) => &plan.events.iter().find(|event| event.id == id)?.trigger,
        Node::Income(id) => match plan.income_source(id)?.span().first() {
            Some(trigger) => trigger,
            None => return Some(plan.plan.start_year),
        },
    };
    resolve_form(plan, trigger.form().ok()?, |node| {
        resolve_node(plan, node, depth + 1)
    })
}

fn resolve_form(
    plan: &Plan,
    form: TriggerForm<'_>,
    year_of: impl Fn(Node<'_>) -> Option<i16>,
) -> Option<i16> {
    match form {
        TriggerForm::Date(date) => Some(date.year()),
        TriggerForm::Age { owner, years } => plan
            .person(owner)
            .map(|person| person.birth.year() + i16::from(years)),
        TriggerForm::Event { id, offset } => Some(year_of(Node::Event(id))? + offset as i16),
        TriggerForm::Income { id, offset } => Some(year_of(Node::Income(id))? + offset as i16),
    }
}
