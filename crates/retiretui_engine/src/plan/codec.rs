use serde::Serialize;
use serde::de::DeserializeOwned;
use toml::Table;

use super::PlanError;

/// A plan value as a TOML table. Both directions go through TOML text:
/// `toml::Value`'s in-memory (de)serializers stringify datetimes, so a
/// direct conversion loses dates.
///
/// # Errors
///
/// Returns [`PlanError`] when the value does not serialize to a table.
pub fn to_table<T: Serialize>(value: &T) -> Result<Table, PlanError> {
    Ok(toml::to_string(value)?.parse()?)
}

/// The plan value a table describes, through TOML text as [`to_table`].
///
/// # Errors
///
/// Returns [`PlanError`] when the table does not match `T`'s schema.
pub fn from_table<T: DeserializeOwned>(table: &Table) -> Result<T, PlanError> {
    Ok(toml::from_str(&toml::to_string(table)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::PlanDate;

    #[test]
    fn a_date_survives_the_round_trip() {
        #[derive(Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Dated {
            on: PlanDate,
        }
        let dated = Dated {
            on: PlanDate(jiff::civil::date(2030, 6, 15)),
        };
        assert_eq!(
            from_table::<Dated>(&to_table(&dated).unwrap()).unwrap(),
            dated
        );
    }
}
