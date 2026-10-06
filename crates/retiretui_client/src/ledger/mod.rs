//! What the Ledger says: every year in a table, one year in full, and the
//! years each earner's salary ends.

mod flows;
mod funds;
mod history;
mod salary;
mod table;
mod tax;
mod year;

#[cfg(test)]
mod table_tests;
#[cfg(test)]
mod tests;

pub use flows::{AccountFlows, FLOW_COLUMNS, FLOWS, account_flows, all_accounts};
pub use funds::{DetailLine, Funds, MONEY_IN, MONEY_OUT, money_in, money_out};
pub use history::History;
pub use salary::{salary_ends, salary_marks};
pub use table::{ColumnSet, Marks, Table, TableRow, YEARS};
pub use tax::TAX;
pub use year::{Asked, Year};

/// What the Ledger titles what a year has the household do.
pub const TO_DO: &str = "To do";
