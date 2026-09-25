use serde::{Deserialize, Serialize};

use super::dates::PlanDate;

/// When something happens in a plan.
///
/// Exactly one basis must be set: `date`, `age` (with `owner`), `event`, or
/// `income`. `offset` shifts the reference forms by whole years and is only
/// valid on `event` and `income`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trigger {
    /// Fires in the year containing this date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<PlanDate>,
    /// Fires in the year the owner reaches this age.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<u8>,
    /// The person `age` refers to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Fires when the named event fires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Fires when the named income source starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub income: Option<String>,
    /// Whole-year shift applied to `event` or `income`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,
}

/// A validated view of a [`Trigger`], one variant per basis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerForm<'a> {
    /// The year containing a fixed date.
    Date(PlanDate),
    /// The year a person reaches an age.
    Age {
        /// Person id.
        owner: &'a str,
        /// Age in whole years.
        years: u8,
    },
    /// A named event's year, shifted by `offset`.
    Event {
        /// Event id.
        id: &'a str,
        /// Whole-year shift.
        offset: i32,
    },
    /// A named income source's start year, shifted by `offset`.
    Income {
        /// Income id.
        id: &'a str,
        /// Whole-year shift.
        offset: i32,
    },
}

/// What a trigger is measured from: the one key every trigger states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerBasis {
    /// A fixed date.
    Date,
    /// A person's age.
    Age,
    /// A named event.
    Event,
    /// A named income source's start.
    Income,
}

/// One key of a [`Trigger`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operand {
    /// `date`: a calendar date.
    Date,
    /// `age`: whole years.
    Years,
    /// `owner`: the id of the person an age is measured on.
    Owner,
    /// `event`: an event id.
    EventId,
    /// `income`: an income source id.
    IncomeId,
    /// `offset`: a whole-year shift; optional.
    Offset,
}

impl TriggerBasis {
    /// Every basis, in the order the schema declares them.
    pub const ALL: &'static [Self] = &[Self::Date, Self::Age, Self::Event, Self::Income];

    /// The key a plan file states the basis under.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.operands()[0].key()
    }

    /// The keys a trigger on this basis may state, the basis's own first.
    #[must_use]
    pub const fn operands(self) -> &'static [Operand] {
        match self {
            Self::Date => &[Operand::Date],
            Self::Age => &[Operand::Years, Operand::Owner],
            Self::Event => &[Operand::EventId, Operand::Offset],
            Self::Income => &[Operand::IncomeId, Operand::Offset],
        }
    }
}

impl Operand {
    /// The key as a plan file spells it.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Date => "date",
            Self::Years => "age",
            Self::Owner => "owner",
            Self::EventId => "event",
            Self::IncomeId => "income",
            Self::Offset => "offset",
        }
    }
}

const ONE_BASIS: &str = "exactly one of `date`, `age`, `event`, or `income` is required";

impl Trigger {
    fn states(&self, basis: TriggerBasis) -> bool {
        match basis {
            TriggerBasis::Date => self.date.is_some(),
            TriggerBasis::Age => self.age.is_some(),
            TriggerBasis::Event => self.event.is_some(),
            TriggerBasis::Income => self.income.is_some(),
        }
    }

    /// The one basis the trigger states.
    ///
    /// # Errors
    ///
    /// Returns a message when no basis or more than one basis is set.
    pub fn basis(&self) -> Result<TriggerBasis, String> {
        let mut stated = TriggerBasis::ALL
            .iter()
            .copied()
            .filter(|basis| self.states(*basis));
        match (stated.next(), stated.next()) {
            (Some(basis), None) => Ok(basis),
            _ => Err(ONE_BASIS.into()),
        }
    }

    /// Classifies the trigger into exactly one [`TriggerForm`].
    ///
    /// # Errors
    ///
    /// Returns a message when no basis or more than one basis is set, when
    /// `age` and `owner` do not appear together, or when `offset` accompanies
    /// a `date` or `age` basis.
    pub fn form(&self) -> Result<TriggerForm<'_>, String> {
        let basis = self.basis()?;
        let takes = |operand| basis.operands().contains(&operand);
        if self.owner.is_some() && !takes(Operand::Owner) {
            return Err("`owner` is only valid together with `age`".into());
        }
        if self.offset.is_some() && !takes(Operand::Offset) {
            return Err("`offset` is only valid with `event` or `income`".into());
        }
        let offset = self.offset.unwrap_or(0);
        let form = match basis {
            TriggerBasis::Date => self.date.map(TriggerForm::Date),
            TriggerBasis::Age => {
                let owner = self
                    .owner
                    .as_deref()
                    .ok_or_else(|| "`age` requires an `owner`".to_owned())?;
                self.age.map(|years| TriggerForm::Age { owner, years })
            }
            TriggerBasis::Event => self
                .event
                .as_deref()
                .map(|id| TriggerForm::Event { id, offset }),
            TriggerBasis::Income => self
                .income
                .as_deref()
                .map(|id| TriggerForm::Income { id, offset }),
        };
        form.ok_or_else(|| ONE_BASIS.into())
    }
}
