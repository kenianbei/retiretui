use retiretui_engine::params::TaxTables;

use super::tests::{full, plans};
use super::*;
use crate::actions::collect_warnings;
use crate::overview::milestones;
use crate::present::treatment_class;
use crate::table::{Column, present_classes, year_figures};

#[test]
fn a_year_is_marked_where_it_has_a_milestone_or_a_warning() {
    let tables = TaxTables::embedded();
    let (mut milestone_years, mut warned_years) = (0, 0);
    for projected in plans() {
        let table = Table::new(&projected, &tables, ColumnSet::Treatments, true);
        let dated = milestones(&projected, true);
        let years = projected.projection.years.iter();
        for (row, said) in years.zip(&table.rows) {
            let is_milestone = dated.iter().any(|each| each.year == Some(row.year));
            let warnings = collect_warnings(&projected.plan, &tables, row, None);
            let wanted = Marks {
                is_milestone,
                has_warning: !warnings.is_empty(),
            };
            assert_eq!(said.marks, wanted, "{}", row.year);
            assert_eq!(said.is_exceeded, row.unfunded > 0);
            milestone_years += usize::from(is_milestone);
            warned_years += usize::from(wanted.has_warning);
        }
    }
    assert!(milestone_years > 0 && warned_years > 0);
}

#[test]
fn the_marked_year_either_side_stops_at_the_plan_s_ends() {
    let projected = full();
    let table = Table::new(&projected, &TaxTables::embedded(), ColumnSet::Tax, true);
    let marked: Vec<i16> = (table.rows.iter())
        .filter(|row| row.marks.is_marked())
        .map(|row| row.year)
        .collect();
    let (first, last) = (marked[0], *marked.last().unwrap());
    assert!(
        marked.len() > 2 && marked.len() < table.rows.len(),
        "{marked:?}"
    );
    assert_eq!(table.marked_year(first, -1), None);
    assert_eq!(table.marked_year(last, 1), None);
    assert_eq!(table.marked_year(first, 1), Some(marked[1]));
    assert_eq!(table.marked_year(marked[1], -1), Some(first));
    assert_eq!(table.marked_year(i16::MIN, 1), Some(first));
    assert_eq!(table.marked_year(i16::MAX, -1), Some(last));
    let unmarked = table
        .rows
        .iter()
        .find(|row| !row.marks.is_marked())
        .unwrap();
    let later = table.marked_year(unmarked.year, 1);
    assert!(later.is_none_or(|year| year > unmarked.year), "{later:?}");
}

#[test]
fn every_column_set_heads_each_figure_it_holds() {
    let projected = full();
    let tables = TaxTables::embedded();
    for set in ColumnSet::ALL {
        for is_nominal in [true, false] {
            let table = Table::new(&projected, &tables, set, is_nominal);
            let figures = table.figure_headers.len();
            assert_eq!(table.text_headers, ["Year", "Age"]);
            assert!(
                table.rows.iter().all(|row| row.figures.len() == figures),
                "{set:?}"
            );
            let leading = &table.figure_headers[..Table::LEADING];
            assert_eq!(
                leading,
                ["Income", "Spending", "Tax", "Withdrawn"],
                "{set:?}"
            );
            assert_eq!(table.figure_headers.last().unwrap(), "Net worth", "{set:?}");
        }
    }
}

#[test]
fn the_treatment_set_is_the_table_the_ledger_always_had() {
    let projected = full();
    let tables = TaxTables::embedded();
    let treated = Table::new(&projected, &tables, ColumnSet::Treatments, true);
    let classes = present_classes(&projected.plan);
    let named = classes.iter().map(|&class| treatment_class(class));
    let figures = ["Income", "Spending", "Tax", "Withdrawn"].into_iter();
    let wanted: Vec<&str> = figures.chain(named).chain(["Net worth"]).collect();
    assert_eq!(treated.figure_headers, wanted);
    let columns: Vec<Column> = classes.into_iter().map(Column::Class).collect();
    for (row, said) in projected.projection.years.iter().zip(&treated.rows) {
        assert_eq!(said.figures, year_figures(row, &columns), "{}", row.year);
    }
}

#[test]
fn the_account_and_tax_sets_hold_their_own_figures() {
    let projected = full();
    let tables = TaxTables::embedded();
    let by_account = Table::new(&projected, &tables, ColumnSet::Accounts, true);
    assert_eq!(
        by_account.figure_headers.len(),
        Table::LEADING + projected.plan.accounts.len() + 1
    );
    let row = &projected.projection.years[11];
    let tax = Table::new(&projected, &tables, ColumnSet::Tax, true);
    let taxed = &tax.rows[11];
    let named: Vec<&str> = vec!["MAGI", "Taxable inc", "Converted", "RMDs"];
    let heads = &tax.figure_headers[Table::LEADING..Table::LEADING + 4];
    assert_eq!(heads, named);
    assert_eq!(
        taxed.figures[4..8],
        [
            row.taxes.magi,
            row.taxes.ordinary_taxable,
            row.conversions,
            row.rmds
        ]
    );
    assert_eq!(
        (taxed.figures[3], taxed.figures[8]),
        (row.total_withdrawals(), row.net_worth)
    );
    let todays = &Table::new(&projected, &tables, ColumnSet::Tax, false).rows[11];
    assert!(
        todays.figures[4] < taxed.figures[4],
        "figures follow the basis"
    );
}

#[test]
fn the_column_sets_turn_in_a_ring() {
    assert_eq!(ColumnSet::default(), ColumnSet::Treatments);
    assert_eq!(ColumnSet::Treatments.neighbor(1), ColumnSet::Accounts);
    assert_eq!(ColumnSet::Tax.neighbor(1), ColumnSet::Treatments);
    assert_eq!(ColumnSet::Treatments.neighbor(-1), ColumnSet::Tax);
    let slugs: Vec<&str> = ColumnSet::ALL.iter().map(|set| set.slug()).collect();
    assert_eq!(slugs, ["treatments", "accounts", "tax"]);
}
