//! What an account is invested in: its share of each asset class, held
//! throughout or changed in steps as triggers fire.

use serde::{Deserialize, Serialize};

use super::triggers::Trigger;
use super::validate::push_issue;
use super::{Account, Issue};

/// How far a mix's shares may sum from one before it is refused.
const SHARE_TOLERANCE: f64 = 1e-6;

/// The asset classes a market moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetClass {
    /// Equities.
    Stocks,
    /// Government bonds.
    Bonds,
    /// Short-term deposits and bills.
    Cash,
}

impl AssetClass {
    /// Every class, in the order the schema declares them.
    pub const ALL: &'static [Self] = &[Self::Stocks, Self::Bonds, Self::Cash];

    /// The class as a plan file spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stocks => "stocks",
            Self::Bonds => "bonds",
            Self::Cash => "cash",
        }
    }

    /// Where the class sits in a per-class array such as a year's returns.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// A year's return on each asset class, indexed by [`AssetClass::index`].
pub type ClassReturns = [f64; 3];

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
fn is_zero(share: &f64) -> bool {
    *share == 0.0
}

/// An account's share of each class, rebalanced every year; an unstated
/// share is none.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mix {
    /// The share in stocks.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub stocks: f64,
    /// The share in bonds.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bonds: f64,
    /// The share in cash.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cash: f64,
}

impl Mix {
    /// The share held in `class`.
    #[must_use]
    pub fn share(&self, class: AssetClass) -> f64 {
        match class {
            AssetClass::Stocks => self.stocks,
            AssetClass::Bonds => self.bonds,
            AssetClass::Cash => self.cash,
        }
    }

    /// The year's return on the whole mix.
    #[must_use]
    pub fn blend(&self, returns: &ClassReturns) -> f64 {
        AssetClass::ALL
            .iter()
            .map(|&class| self.share(class) * returns[class.index()])
            .sum()
    }

    fn check(&self, path: &str, issues: &mut Vec<Issue>) {
        let shares = AssetClass::ALL.iter().map(|&class| self.share(class));
        if shares
            .clone()
            .any(|share| !share.is_finite() || !(0.0..=1.0).contains(&share))
        {
            push_issue(issues, path, "each share must be between 0 and 1");
            return;
        }
        if (shares.sum::<f64>() - 1.0).abs() > SHARE_TOLERANCE {
            push_issue(issues, path, "the shares must add up to 1");
        }
    }
}

/// One step of a glide path: the mix held from `from` until the next step
/// fires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MixPhase {
    /// When this mix takes over.
    pub from: Trigger,
    /// The share in stocks.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub stocks: f64,
    /// The share in bonds.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bonds: f64,
    /// The share in cash.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cash: f64,
}

impl MixPhase {
    /// The mix this step holds.
    #[must_use]
    pub fn mix(&self) -> Mix {
        Mix {
            stocks: self.stocks,
            bonds: self.bonds,
            cash: self.cash,
        }
    }
}

/// What an account is invested in: one mix throughout, or a glide path of
/// steps, each holding from its trigger until a later one fires and the
/// first holding until then.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Allocation {
    /// Steps from triggers. Tried first: a mix would also read a list.
    GlidePath(Vec<MixPhase>),
    /// One mix throughout.
    Mix(Mix),
}

/// Checks how an account says what it earns: a rate, or a mix, not both.
pub(super) fn check_returns(path: &str, account: &Account, issues: &mut Vec<Issue>) {
    let rate_path = format!("{path}.expected_return");
    if let Some(rate) = account.expected_return
        && (!rate.is_finite() || !(-1.0..=1.0).contains(&rate))
    {
        push_issue(issues, &rate_path, "must be a rate between -1.0 and 1.0");
    }
    let Some(allocation) = &account.allocation else {
        return;
    };
    if account.expected_return.is_some() {
        push_issue(
            issues,
            &rate_path,
            "an account with an allocation earns its mix; remove the expected return",
        );
    }
    let path = format!("{path}.allocation");
    match allocation {
        Allocation::Mix(mix) => mix.check(&path, issues),
        Allocation::GlidePath(phases) if phases.is_empty() => {
            push_issue(issues, &path, "a glide path needs at least one step");
        }
        Allocation::GlidePath(phases) => {
            for (i, phase) in phases.iter().enumerate() {
                phase.mix().check(&format!("{path}[{i}]"), issues);
            }
        }
    }
}
