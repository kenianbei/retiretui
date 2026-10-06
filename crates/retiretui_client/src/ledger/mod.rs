//! What the Ledger says: every year in a table, one year in full, and the
//! years each earner's salary ends.

mod flows;
mod former;
mod funds;
mod salary;
mod table;
mod tax;
mod year;

#[cfg(test)]
mod tests;

pub use flows::{AccountFlows, FLOW_COLUMNS, FLOWS, account_flows, all_accounts};
pub use former::{FLOW_HEADERS, INCOME_AND_TAX, income_and_tax, ledger_headers};
pub use funds::{DetailLine, Funds, Group, MONEY_IN, MONEY_OUT, money_in, money_out};
pub use salary::{salary_ends, salary_marks};
pub use table::{ColumnSet, Marks, Table, TableRow, YEARS};
pub use tax::TAX;
pub use year::{Asked, Year};

/// What the Ledger titles what a year has the household do.
pub const TO_DO: &str = "To do";
