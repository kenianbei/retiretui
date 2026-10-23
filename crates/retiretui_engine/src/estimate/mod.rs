//! The plan's expenses estimated from a bank's exports: each row sorted
//! into a category by the bank's own column, its transaction type or its
//! payee's words, the latest twelve months' spending in each annualized in
//! the plan's dollars, and each category placed, as the plan's
//! `[estimate]` table leans, in the expense that is for it, one of its
//! family, a new one, or the bucket - one proposal per expense.

mod bank;
mod category;
mod csv;
mod ofx;
mod patterns;
mod payee;
mod place;

use std::collections::{BTreeMap, HashMap};

pub use bank::{BankError, CENTS_PER_DOLLAR, Cents, Row, merge, read};
pub use category::{Category, Family};
pub use patterns::Patterns;
pub use payee::normalize;
pub use place::Why;

use serde::{Deserialize, Serialize};

use crate::params::scale;
use crate::plan::{ColaSpec, Dollars, Expense, Item, Plan, PlanDate, Trigger};
use crate::project::{inflation_factor, trigger_month};
use crate::tax::month_index;

const MONTHS_PER_YEAR: i32 = 12;

/// What the exports say the plan spends.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Estimate {
    /// The whole months the window covers, twelve at most.
    pub months: u8,
    /// The latest row's date: the window ends with its month.
    #[cfg_attr(feature = "schemars", schemars(with = "Option<String>"))]
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub through: Option<PlanDate>,
    /// One per spending category seen, in the vocabulary's order.
    pub proposals: Vec<Proposal>,
    /// Each payee nothing named, largest first.
    pub unknown: Vec<Unknown>,
    /// What came in over the window, annualized in the plan's dollars.
    pub income: Dollars,
    /// What moved without being spent over the window, net, annualized.
    pub transfers: Dollars,
}

/// What the spending landing in one expense comes to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Proposal {
    /// The plan expense it sets, or the id a new one is added under.
    pub id: String,
    /// Whether it adds an expense rather than setting one the plan holds.
    pub is_new: bool,
    /// The expense's name, or the one a new expense is given.
    pub name: String,
    /// Whether it collects what fits nowhere else.
    pub is_bucket: bool,
    /// What the plan's expense spends a year over the window, as the plan
    /// states it; none for a new one.
    pub now: Option<Dollars>,
    /// Annual, in the plan's start-year dollars.
    pub amount: Dollars,
    /// What the rows add up to over the window, in the same dollars.
    pub total: Dollars,
    /// How many rows.
    pub rows: usize,
    /// Each category it gathers, largest first.
    pub categories: Vec<Gathered>,
    /// What the payees nothing names add up to a year, where it is the
    /// bucket.
    pub unknown: Dollars,
    /// Each payee and what it adds a year, largest first.
    pub payees: Vec<(String, Dollars)>,
    /// The latest row's date.
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub last: PlanDate,
    /// Added as a one-time expense on `last` for `total`, not annualized.
    pub one_time: bool,
    /// Which of a phased expense's amounts the proposal sets.
    pub phases: Phases,
}

/// A category's spending gathered into a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Gathered {
    /// The category.
    pub category: Category,
    /// What it adds a year.
    pub amount: Dollars,
    /// Why it landed in this expense.
    pub why: Why,
}

/// What a proposal does to an expense stated in phases.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Phases {
    /// The amount in force over the window alone.
    #[default]
    This,
    /// Every amount, by the factor the one in force changes by.
    Scaled,
    /// Every amount, to the proposed one.
    All,
}

/// A payee the patterns did not name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Unknown {
    /// The normalized payee.
    pub payee: String,
    /// Annualized, in the plan's dollars, negative where money came in.
    pub amount: Dollars,
    /// How many rows.
    pub rows: usize,
}

/// Deflated dollars under one category or payee.
#[derive(Default)]
struct Tally {
    total: f64,
    rows: usize,
    payees: BTreeMap<String, f64>,
    last: Option<PlanDate>,
}

impl Tally {
    fn absorb(&mut self, other: Self) {
        self.total += other.total;
        self.rows += other.rows;
        for (payee, dollars) in other.payees {
            *self.payees.entry(payee).or_default() += dollars;
        }
        self.last = self.last.max(other.last);
    }
}

/// The tallies landing in one target, and what of them each category and
/// the unknown payees bring.
#[derive(Default)]
struct Group {
    tally: Tally,
    categories: Vec<(Category, f64, Why)>,
    unknown: f64,
}

/// Whose transactions [`records`] lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Of {
    /// The rows sorted into a category.
    Category(Category),
    /// The rows of one payee nothing names, normalized.
    Payee(String),
    /// The rows of every payee nothing names.
    Unnamed,
}

/// The latest twelve whole months the rows reach, as month indexes.
#[derive(Debug, Clone, Copy)]
struct Window {
    start: i32,
    end: i32,
}

fn month_of(on: PlanDate) -> i32 {
    month_index(on.year(), on.0.month())
}

impl Window {
    fn of(rows: &[Row]) -> Option<Self> {
        let end = rows.iter().map(|row| month_of(row.on)).max()?;
        let earliest = rows.iter().map(|row| month_of(row.on)).min().unwrap_or(end);
        Some(Self {
            start: earliest.max(end - (MONTHS_PER_YEAR - 1)),
            end,
        })
    }

    const fn months(self) -> i32 {
        self.end - self.start + 1
    }

    fn holds(self, row: &Row) -> bool {
        (self.start..=self.end).contains(&month_of(row.on))
    }

    fn annual(self, dollars: f64) -> Dollars {
        (dollars * f64::from(MONTHS_PER_YEAR) / f64::from(self.months())).round() as Dollars
    }
}

/// Sorts rows by the bank's own column, their transaction type, or what
/// their payee's words say, each payee normalized and matched once.
struct Sorter<'a> {
    patterns: &'a Patterns,
    named: HashMap<&'a str, (String, Option<Category>)>,
}

impl<'a> Sorter<'a> {
    fn new(patterns: &'a Patterns) -> Self {
        Self {
            patterns,
            named: HashMap::new(),
        }
    }

    /// The row's normalized payee and its category.
    fn sort(&mut self, row: &'a Row) -> (&str, Option<Category>) {
        let patterns = self.patterns;
        let (payee, matched) = self.named.entry(&row.payee).or_insert_with(|| {
            let payee = normalize(&row.payee);
            let matched = patterns.category(&payee);
            (payee, matched)
        });
        let category = row
            .category
            .as_deref()
            .and_then(Category::from_text)
            .or(row.kind)
            .or(*matched);
        (payee, category)
    }
}

/// The rows within the estimate's window that are `of`'s, newest first.
#[must_use]
pub fn records<'a>(rows: &'a [Row], patterns: &Patterns, of: &Of) -> Vec<&'a Row> {
    let Some(window) = Window::of(rows) else {
        return Vec::new();
    };
    let mut sorter = Sorter::new(patterns);
    let mut is_of = |row: &'a Row| match (of, sorter.sort(row)) {
        (Of::Category(wanted), (_, category)) => category == Some(*wanted),
        (Of::Payee(wanted), (payee, None)) => payee == wanted,
        (Of::Payee(_), (_, Some(_))) => false,
        (Of::Unnamed, (_, category)) => category.is_none(),
    };
    let mut found: Vec<&Row> = rows
        .iter()
        .filter(|row| window.holds(row) && is_of(row))
        .collect();
    found.sort_by_key(|row| std::cmp::Reverse(row.on));
    found
}

/// What `rows` say the plan spends, the latest twelve whole months taken
/// and deflated to the plan's start-year dollars by its inflation.
#[must_use]
pub fn estimate(plan: &Plan, rows: &[Row], patterns: &Patterns) -> Estimate {
    let (Some(window), Some(through)) = (Window::of(rows), rows.iter().map(|row| row.on).max())
    else {
        return Estimate::default();
    };
    let in_window = rows.iter().filter(|row| window.holds(row));
    let (mut by_category, unknown) = tally(plan, in_window, patterns);
    let mut listed: Vec<Unknown> = unknown
        .iter()
        .map(|(payee, tally)| Unknown {
            payee: payee.clone(),
            amount: window.annual(-tally.total),
            rows: tally.rows,
        })
        .collect();
    let income = by_category.remove(&Category::Income);
    let transfers = by_category.remove(&Category::Transfer);
    let (groups, bucket) = grouped(plan, by_category, unknown);
    let proposals = groups
        .into_iter()
        .map(|(target, group)| proposal(plan, (target, target == bucket), group, window))
        .collect();
    listed
        .sort_by(|a, b| (b.amount.abs().cmp(&a.amount.abs())).then_with(|| a.payee.cmp(&b.payee)));
    let tallied = |tally: Option<Tally>| tally.map_or(0, |t| window.annual(t.total));
    Estimate {
        months: window.months() as u8,
        through: Some(through),
        proposals,
        unknown: listed,
        income: tallied(income),
        transfers: tallied(transfers),
    }
}

/// Each spending category's net spending, and the unknown payees', in the
/// target the plan's `[estimate]` places it in, and which target is the
/// bucket.
fn grouped(
    plan: &Plan,
    by_category: BTreeMap<Category, Tally>,
    unknown: BTreeMap<String, Tally>,
) -> (BTreeMap<place::Target, Group>, place::Target) {
    let spent = |tally: &Tally| (-tally.total).max(0.0);
    let spending = by_category
        .values()
        .chain(unknown.values())
        .map(spent)
        .sum();
    let placer = place::Placer::new(plan, spending);
    let mut groups: BTreeMap<place::Target, Group> = BTreeMap::new();
    for (category, tally) in by_category.into_iter().filter(|(_, t)| t.total < 0.0) {
        let (target, why) = placer.place(category, spent(&tally));
        let group = groups.entry(target).or_default();
        group.categories.push((category, -tally.total, why));
        group.tally.absorb(tally);
    }
    for tally in unknown.into_values().filter(|t| t.total < 0.0) {
        let group = groups.entry(placer.bucket()).or_default();
        group.unknown -= tally.total;
        group.tally.absorb(tally);
    }
    (groups, placer.bucket())
}

/// Each row's deflated amount under its category, or under its payee
/// where nothing names one; a payee is normalized and matched once.
fn tally<'a>(
    plan: &Plan,
    rows: impl Iterator<Item = &'a Row>,
    patterns: &Patterns,
) -> (BTreeMap<Category, Tally>, BTreeMap<String, Tally>) {
    let mut by_category: BTreeMap<Category, Tally> = BTreeMap::new();
    let mut unknown: BTreeMap<String, Tally> = BTreeMap::new();
    let mut sorter = Sorter::new(patterns);
    for row in rows {
        let (payee, category) = sorter.sort(row);
        let payee = payee.to_owned();
        let deflated =
            row.amount.0 as f64 / CENTS_PER_DOLLAR as f64 / inflation_factor(plan, row.on.year());
        let tally = match category {
            Some(category) => by_category.entry(category).or_default(),
            None => unknown.entry(payee.clone()).or_default(),
        };
        tally.total += deflated;
        tally.rows += 1;
        *tally.payees.entry(payee).or_default() += deflated;
        tally.last = tally.last.max(Some(row.on));
    }
    (by_category, unknown)
}

/// Largest first, ties by their key.
fn largest_first<K: Ord>(a: &(K, Dollars), b: &(K, Dollars)) -> std::cmp::Ordering {
    b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0))
}

fn proposal(
    plan: &Plan,
    (target, is_bucket): (place::Target, bool),
    group: Group,
    window: Window,
) -> Proposal {
    let mut payees: Vec<(String, Dollars)> = group
        .tally
        .payees
        .into_iter()
        .map(|(payee, dollars)| (payee, window.annual(-dollars)))
        .collect();
    payees.sort_by(largest_first);
    let mut categories: Vec<(Category, Dollars, Why)> = group
        .categories
        .into_iter()
        .map(|(category, dollars, why)| (category, window.annual(dollars), why))
        .collect();
    categories.sort_by(|a, b| largest_first(&(a.0, a.1), &(b.0, b.1)));
    let (id, now, name) = identified(plan, target, window.end);
    Proposal {
        id,
        is_new: now.is_none(),
        name,
        is_bucket,
        now,
        amount: window.annual(-group.tally.total),
        total: (-group.tally.total).round() as Dollars,
        rows: group.tally.rows,
        categories: categories
            .into_iter()
            .map(|(category, amount, why)| Gathered {
                category,
                amount,
                why,
            })
            .collect(),
        unknown: window.annual(group.unknown),
        payees,
        last: group.tally.last.expect("a group holds a row"),
        one_time: false,
        phases: Phases::default(),
    }
}

/// The id, the amount in force at `end` and the name of the expense
/// `target` sets, or of the one it adds.
fn identified(plan: &Plan, target: place::Target, end: i32) -> (String, Option<Dollars>, String) {
    match target {
        place::Target::Expense(at) => {
            let stated = &plan.expenses[at];
            let now = in_force_amount(plan, stated, end);
            (
                stated.id.clone(),
                Some(now),
                stated.display_name().to_owned(),
            )
        }
        place::Target::New(category) => (category.slug().to_owned(), None, category.title()),
        place::Target::NewBucket => (
            place::NEW_BUCKET_ID.to_owned(),
            None,
            place::NEW_BUCKET_NAME.to_owned(),
        ),
    }
}

/// The expense what fits nowhere else goes to: the one the plan's
/// `[estimate]` names, else one whose id or name says misc, other or
/// general; none where a new one would be proposed.
#[must_use]
pub fn bucket(plan: &Plan) -> Option<&Expense> {
    place::bucket_position(plan).map(|at| &plan.expenses[at])
}

/// Sets each proposal's expense to its amount, or adds the expense the plan
/// has none for; the proposals left as they came set nothing they did not
/// propose.
pub fn apply(plan: &mut Plan, estimate: &Estimate) {
    let end = estimate.through.map(month_of);
    for proposal in &estimate.proposals {
        let at = (!proposal.is_new)
            .then(|| plan.expenses.iter().position(|e| e.id == proposal.id))
            .flatten();
        match at {
            Some(at) => {
                let in_force = end.and_then(|end| phase_in_force(plan, &plan.expenses[at], end));
                set_amount(&mut plan.expenses[at], proposal, in_force);
            }
            None => plan.expenses.push(new_expense(proposal)),
        }
    }
}

/// What `expense` spends a year in the month `end`.
fn in_force_amount(plan: &Plan, expense: &Expense, end: i32) -> Dollars {
    phase_in_force(plan, expense, end).map_or(expense.amount, |at| expense.phases[at].amount)
}

/// The index of the last phase fired by the window's `end` month, or none
/// where the base amount is still in force.
fn phase_in_force(plan: &Plan, expense: &Expense, end: i32) -> Option<usize> {
    expense
        .phases
        .iter()
        .rposition(|phase| trigger_month(plan, &phase.from).is_some_and(|month| month <= end))
}

fn set_amount(expense: &mut Expense, proposal: &Proposal, in_force: Option<usize>) {
    let current = in_force.map_or(expense.amount, |at| expense.phases[at].amount);
    let scale = |amount: Dollars| {
        if current == 0 {
            proposal.amount
        } else {
            scale(amount, proposal.amount as f64 / current as f64)
        }
    };
    match proposal.phases {
        Phases::This => match in_force {
            Some(at) => expense.phases[at].amount = proposal.amount,
            None => expense.amount = proposal.amount,
        },
        Phases::Scaled => {
            expense.amount = scale(expense.amount);
            for phase in &mut expense.phases {
                phase.amount = scale(phase.amount);
            }
        }
        Phases::All => {
            expense.amount = proposal.amount;
            for phase in &mut expense.phases {
                phase.amount = proposal.amount;
            }
        }
    }
}

fn new_expense(proposal: &Proposal) -> Expense {
    Expense {
        id: proposal.id.clone(),
        name: Some(proposal.name.clone()),
        amount: if proposal.one_time {
            proposal.total
        } else {
            proposal.amount
        },
        start: None,
        end: None,
        on: proposal.one_time.then_some(Trigger {
            date: Some(proposal.last),
            age: None,
            owner: None,
            event: None,
            income: None,
            offset: None,
        }),
        cola: ColaSpec::default(),
        medical: false,
        essential: false,
        phases: Vec::new(),
    }
}
