use jiff::civil::Date;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A calendar date in a plan file, written as a native TOML date
/// (`1975-06-14`, no time or offset component).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlanDate(pub Date);

impl PlanDate {
    /// The calendar year.
    #[must_use]
    pub fn year(self) -> i16 {
        self.0.year()
    }
}

impl<'de> Deserialize<'de> for PlanDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = toml::value::Datetime::deserialize(deserializer)?;
        let date = raw
            .date
            .ok_or_else(|| D::Error::custom("expected a calendar date"))?;
        if raw.time.is_some() || raw.offset.is_some() {
            return Err(D::Error::custom(
                "expected a plain date without a time or offset",
            ));
        }
        let year = i16::try_from(date.year).map_err(D::Error::custom)?;
        Date::new(year, date.month as i8, date.day as i8)
            .map(PlanDate)
            .map_err(D::Error::custom)
    }
}

impl Serialize for PlanDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        toml::value::Datetime {
            date: Some(toml::value::Date {
                year: self.0.year() as u16,
                month: self.0.month() as u8,
                day: self.0.day() as u8,
            }),
            time: None,
            offset: None,
        }
        .serialize(serializer)
    }
}

/// A [`PlanDate`] as `YYYY-MM-DD` text, for `#[serde(with)]` on a reply read
/// as JSON, where the native TOML date would be an object.
pub(crate) mod text {
    use serde::de::Error as _;

    use super::{Deserialize, Deserializer, PlanDate, Serializer};

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's `with` signature"
    )]
    pub(crate) fn serialize<S: Serializer>(
        date: &PlanDate,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&date.0)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<PlanDate, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map(PlanDate).map_err(D::Error::custom)
    }
}

/// [`text`], or null where there is no date.
pub(crate) mod optional_text {
    use serde::de::Error as _;

    use super::{Deserialize, Deserializer, PlanDate, Serializer, text};

    #[expect(clippy::ref_option, reason = "serde's `with` signature")]
    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's `with` signature; the lint's size limit varies by target"
    )]
    pub(crate) fn serialize<S: Serializer>(
        date: &Option<PlanDate>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match date {
            Some(date) => text::serialize(date, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<PlanDate>, D::Error> {
        let text = Option::<String>::deserialize(deserializer)?;
        text.map(|text| text.parse().map(PlanDate).map_err(D::Error::custom))
            .transpose()
    }
}
