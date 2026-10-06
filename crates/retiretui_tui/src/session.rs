use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::prelude::{Changed, ChildOf, Component, Entity, Query, Res, ResMut, Resource, With};
use bevy_ecs::system::SystemParam;
use plurimus::widgets::ActiveDescendant;

use crate::command::Outcome;

pub use retiretui_client::session::{
    NO_DOCUMENT, Projected, Session, Today, YearCursor, cursor_year, file_name, span,
};

/// Moves the year cursor `step` years within the projected ones.
fn step_year(
    projected: &Projected,
    today: Today,
    cursor: &mut ResMut<YearCursor>,
    step: i16,
) -> Outcome {
    let planned = span(&projected.projection.years);
    let year = cursor_year(projected, today, **cursor);
    let stepped = (year + step).clamp(planned.0, planned.1);
    if stepped != year {
        cursor.set_if_neq(YearCursor(Some(stepped)));
    }
    Outcome::Done
}

/// Steps the shared year a year on.
pub(crate) fn next_year(
    projected: Res<Projected>,
    today: Res<Today>,
    mut cursor: ResMut<YearCursor>,
) -> Outcome {
    step_year(&projected, *today, &mut cursor, 1)
}

/// Steps the shared year a year back.
pub(crate) fn previous_year(
    projected: Res<Projected>,
    today: Res<Today>,
    mut cursor: ResMut<YearCursor>,
) -> Outcome {
    step_year(&projected, *today, &mut cursor, -1)
}

/// The calendar year a table's row shows.
#[derive(Component, Clone, Copy)]
pub struct RowYear(pub i16);

/// Moving the cursor of the `T` table moves the year cursor to the year of
/// the row it lands on, unless the cursor already resolves to it among the
/// table's years, so a table pointed at the cursor never writes it back.
pub fn track_cursor<T: Component>(
    tables: Query<(Entity, &ActiveDescendant), (With<T>, Changed<ActiveDescendant>)>,
    rows: Query<(&RowYear, &ChildOf)>,
    projected: Res<Projected>,
    today: Res<Today>,
    mut cursor: ResMut<YearCursor>,
) {
    let Some((table, &ActiveDescendant(Some(on)))) = tables.iter().next() else {
        return;
    };
    let Ok((&RowYear(year), _)) = rows.get(on) else {
        return;
    };
    let years = rows.iter().filter(|(_, parent)| parent.parent() == table);
    let shows = years.fold((year, year), |(first, last), (&RowYear(row), _)| {
        (first.min(row), last.max(row))
    });
    let planned = span(&projected.projection.years);
    if cursor.resolve(*today, planned, shows) != year {
        cursor.set_if_neq(YearCursor(Some(year)));
    }
}

/// A market run the Ledger shows in place of the plan's own projection,
/// under the words it is named by, until it is left or the plan changes.
#[derive(Resource, Default)]
pub struct LedgerRun(pub Option<(String, Projected)>);

impl LedgerRun {
    /// What the Ledger shows: the run, else the plan's own `projected`.
    pub fn over<'a>(&'a self, projected: &'a Projected) -> &'a Projected {
        self.0.as_ref().map_or(projected, |(_, run)| run)
    }
}

/// What the views of the projection draw from.
#[derive(SystemParam)]
pub struct Shown<'w> {
    pub projected: Res<'w, Projected>,
    pub basis: Res<'w, Basis>,
    cursor: Res<'w, YearCursor>,
    today: Res<'w, Today>,
    pub run: Res<'w, LedgerRun>,
}

impl Shown<'_> {
    pub fn is_changed(&self) -> bool {
        self.projected.is_changed()
            || self.basis.is_changed()
            || self.cursor.is_changed()
            || self.run.is_changed()
    }

    /// Whether the cursor has moved, which the plan's own views follow
    /// alone of what [`Self::is_changed`] watches beyond the plan and the
    /// basis.
    pub fn is_year_changed(&self) -> bool {
        self.cursor.is_changed()
    }

    /// What the Ledger shows: the run opened in it, else the plan's own.
    pub fn ledger(&self) -> &Projected {
        self.run.over(&self.projected)
    }

    /// The cursor's year among the plan's years, which a run opened in
    /// the Ledger shares.
    pub fn year(&self) -> i16 {
        cursor_year(&self.projected, *self.today, *self.cursor)
    }

    /// The cursor's year among `shows`, years that run past the plan's.
    pub fn year_among(&self, shows: (i16, i16)) -> i16 {
        let planned = span(&self.projected.projection.years);
        self.cursor.resolve(*self.today, planned, shows)
    }
}

/// The displayed dollar basis.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct Basis {
    /// Show future (nominal) dollars instead of today's dollars.
    pub nominal: bool,
}
