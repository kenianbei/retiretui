use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use super::Dollars;
use super::dates::PlanDate;
use super::triggers::Trigger;

/// Federal filing status. These two are the supported statuses; they cover a
/// one- or two-person household and the survivor transition between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FilingStatus {
    /// One filer.
    Single,
    /// Married filing jointly.
    MarriedJoint,
}

impl FilingStatus {
    /// Every status, in the order the schema declares them.
    pub const ALL: &'static [Self] = &[Self::Single, Self::MarriedJoint];

    /// The status as a plan file spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::MarriedJoint => "married-joint",
        }
    }
}

/// A member of the household.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    /// Unique id other plan items reference as `owner`.
    pub id: String,
    /// The person's name as it is shown; the id where there is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Birth date.
    pub birth: PlanDate,
    /// Covered (FICA) earnings by calendar year, nominal, as the Social
    /// Security statement records them; what a `social-security` income
    /// without an `amount` is computed from.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub earnings: BTreeMap<i16, Dollars>,
}

impl Person {
    /// What the person is called where they are shown: their name, or
    /// their id where they have none.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    /// The age this person reaches during the given calendar year.
    #[must_use]
    pub fn age_in_year(&self, year: i16) -> i16 {
        year - self.birth.year()
    }
}

/// The household: filing status and its one or two people.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Household {
    /// Federal filing status.
    pub filing: FilingStatus,
    /// The people; one for `single`, two for `married-joint`.
    pub people: Vec<Person>,
}

/// Where the household lives from `from` until the next residency begins.
/// Validated but not yet priced: the engine models federal tax only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Residency {
    /// ISO 3166-1 two-letter country code, one of
    /// [`COUNTRIES`](super::COUNTRIES); read in any case, kept lowercase.
    #[serde(deserialize_with = "lowercased")]
    pub country: String,
    /// One of [`US_STATES`](super::US_STATES), given exactly when `country`
    /// is `us`; read in any case, kept lowercase.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "lowercased_option"
    )]
    pub state: Option<String>,
    /// When this residency begins; absent means from plan start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Trigger>,
}

fn lowercased<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    String::deserialize(deserializer).map(|code| code.to_ascii_lowercase())
}

fn lowercased_option<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    let code = Option::<String>::deserialize(deserializer)?;
    Ok(code.map(|code| code.to_ascii_lowercase()))
}
