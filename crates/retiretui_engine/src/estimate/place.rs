//! Where each category's spending lands, and why: the expense it is
//! assigned or named for, a new one as the plan's `[estimate]` leans, an
//! expense of its family, or the bucket.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::category::names_any;
use super::{Category, Family};
use crate::plan::{EstimateSettings, Expense, Lean, Plan};

/// The words an expense's id or name holds to be taken for the bucket where
/// the plan names none.
const BUCKET_WORDS: [&str; 3] = ["misc", "other", "general"];
/// The id and name of the bucket proposed where the plan has none.
pub(super) const NEW_BUCKET_ID: &str = "other";
pub(super) const NEW_BUCKET_NAME: &str = "Other";

/// An expense spending lands in: one the plan holds, by its place, or one
/// to add. Ordered as the proposals are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Target {
    Expense(usize),
    New(Category),
    NewBucket,
}

/// Why a category landed where it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Why {
    /// The plan's `[estimate]` assigns it there.
    Assigned,
    /// The expense is named for it.
    Named,
    /// The expense is of its family.
    Family,
    /// Nothing else fits it.
    Bucket,
    /// It is a large enough share of the year's spending for its own.
    Large,
    /// The plan leans to a new expense for every category.
    Always,
}

/// The plan's expenses and `[estimate]` table, read once per estimate.
pub(super) struct Placer<'a> {
    settings: &'a EstimateSettings,
    /// The first expense each category is named for.
    named: BTreeMap<Category, usize>,
    /// The first expense some category of each family is named for.
    kin: BTreeMap<Family, usize>,
    /// Where each assigned category goes.
    assigned: BTreeMap<Category, usize>,
    bucket: Target,
    /// The year's spending, which a category's share is of.
    spending: f64,
}

static UNSTATED: EstimateSettings = EstimateSettings {
    lean: Lean::Large,
    large_share: None,
    bucket: None,
    assign: Vec::new(),
};

impl<'a> Placer<'a> {
    pub(super) fn new(plan: &'a Plan, spending: f64) -> Self {
        let settings = plan.estimate.as_ref().unwrap_or(&UNSTATED);
        let mut named = BTreeMap::new();
        let mut kin = BTreeMap::new();
        for (at, expense) in plan.expenses.iter().enumerate() {
            for category in Category::ALL.into_iter().filter(|c| c.matches(expense)) {
                named.entry(category).or_insert(at);
                if let Some(family) = category.family() {
                    kin.entry(family).or_insert(at);
                }
            }
        }
        let assigned = settings
            .assign
            .iter()
            .filter_map(|it| Some((it.category, position(plan, |e| e.id == it.expense)?)))
            .collect();
        Self {
            settings,
            named,
            kin,
            assigned,
            bucket: bucket_position(plan).map_or(Target::NewBucket, Target::Expense),
            spending,
        }
    }

    /// Where what fits nowhere else goes.
    pub(super) const fn bucket(&self) -> Target {
        self.bucket
    }

    /// Where `category`'s `spent` dollars land, and why.
    pub(super) fn place(&self, category: Category, spent: f64) -> (Target, Why) {
        if let Some(&at) = self.assigned.get(&category) {
            return (Target::Expense(at), Why::Assigned);
        }
        if let Some(&at) = self.named.get(&category) {
            return (Target::Expense(at), Why::Named);
        }
        let is_large = spent >= self.settings.large_share() * self.spending;
        match self.settings.lean {
            Lean::New => return (Target::New(category), Why::Always),
            Lean::Large if is_large => return (Target::New(category), Why::Large),
            Lean::Large | Lean::Existing => {}
        }
        let kin = category.family().and_then(|family| self.kin.get(&family));
        kin.map_or((self.bucket, Why::Bucket), |&at| {
            (Target::Expense(at), Why::Family)
        })
    }
}

/// The place of the expense the plan's `[estimate]` names the bucket, else
/// of one whose name says it is one.
pub(super) fn bucket_position(plan: &Plan) -> Option<usize> {
    let named = plan.estimate.as_ref().and_then(|it| it.bucket.as_deref());
    named
        .and_then(|id| position(plan, |e| e.id == id))
        .or_else(|| position(plan, |e| names_any(e, &BUCKET_WORDS)))
}

fn position(plan: &Plan, is: impl Fn(&Expense) -> bool) -> Option<usize> {
    plan.expenses.iter().position(is)
}
