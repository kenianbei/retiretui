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
