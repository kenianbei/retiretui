//! Where each category's spending lands: the expense it is assigned or
//! named for, a new one as the plan's `[estimate]` leans, an expense of its
//! family, or the bucket.

use super::Category;
use crate::plan::{EstimateSettings, Expense, Item, Lean, Plan};

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

/// The plan's expenses and `[estimate]` table, read once per estimate.
pub(super) struct Placer<'a> {
    plan: &'a Plan,
    settings: EstimateSettings,
    bucket: Target,
    /// The year's spending, which a category's share is of.
    spending: f64,
}

impl<'a> Placer<'a> {
    pub(super) fn new(plan: &'a Plan, spending: f64) -> Self {
        let settings = plan.estimate.clone().unwrap_or_default();
        let bucket = bucket_position(plan).map_or(Target::NewBucket, Target::Expense);
        Self {
            plan,
            settings,
            bucket,
            spending,
        }
    }

    /// Where what fits nowhere else goes.
    pub(super) const fn bucket(&self) -> Target {
        self.bucket
    }

    /// Where `category`'s `spent` dollars land.
    pub(super) fn place(&self, category: Category, spent: f64) -> Target {
        let assigned = self
            .settings
            .assign
            .iter()
            .find(|it| it.category == category);
        if let Some(at) = assigned.and_then(|it| position(self.plan, |e| e.id == it.expense)) {
            return Target::Expense(at);
        }
        if let Some(at) = position(self.plan, |e| category.matches(e)) {
            return Target::Expense(at);
        }
        let is_new = match self.settings.lean {
            Lean::New => true,
            Lean::Large => spent >= self.settings.large_share() * self.spending,
            Lean::Existing => false,
        };
        if is_new {
            return Target::New(category);
        }
        let is_family = |e: &Expense| {
            Category::ALL
                .iter()
                .any(|kin| kin.family() == category.family() && kin.matches(e))
        };
        position(self.plan, is_family).map_or(self.bucket, Target::Expense)
    }
}

/// The place of the expense the plan's `[estimate]` names the bucket, else
/// of one whose name says it is one.
pub(super) fn bucket_position(plan: &Plan) -> Option<usize> {
    let named = plan.estimate.as_ref().and_then(|it| it.bucket.as_deref());
    named
        .and_then(|id| position(plan, |e| e.id == id))
        .or_else(|| position(plan, is_bucket_named))
}

fn position(plan: &Plan, is: impl Fn(&Expense) -> bool) -> Option<usize> {
    plan.expenses.iter().position(is)
}

fn is_bucket_named(expense: &Expense) -> bool {
    let id = expense.id.to_lowercase();
    let name = expense.display_name().to_lowercase();
    BUCKET_WORDS
        .iter()
        .any(|word| id.contains(word) || name.contains(word))
}
