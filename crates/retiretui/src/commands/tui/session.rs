use std::borrow::Cow;
use std::path::{Path, PathBuf};

use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::prelude::{Changed, ChildOf, Component, Entity, Query, Res, ResMut, Resource, With};
use bevy_ecs::system::SystemParam;
use plurimus::widgets::ActiveDescendant;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{Projection, YearRow, project};

use crate::commands::directory_of;

/// Where the shown plan came from and what reloads project against.
#[derive(Resource)]
pub struct Session {
    /// The open plan or scenario; none in the empty shell.
    pub plan_path: Option<PathBuf>,
    /// The directory launched on, or the launched file's.
    launched: PathBuf,
    /// The tax tables loaded at launch.
    pub tables: TaxTables,
}

pub const NO_DOCUMENT: &str = "no document is open";

const NO_DOCUMENT_NAME: &str = "no document";

/// The smallest plan that validates: one person and the cash account
/// surplus lands in, starting `start_year`.
pub fn blank_plan(start_year: i16) -> String {
    BLANK_PLAN.replacen("START_YEAR", &start_year.to_string(), 1)
}

const BLANK_PLAN: &str = r#"schema = 1

[plan]
start_year = START_YEAR
horizon_age = 95
inflation = 0.025

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1970-01-01

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0
"#;

/// The projected plan every view derives from; replaced wholesale on reload.
#[derive(Resource)]
pub struct Projected {
    /// The resolved plan.
    pub plan: Plan,
    /// Its projection.
    pub projection: Projection,
}

impl Session {
    /// A session at `path`: a plan or scenario file, or a directory, which
    /// opens the shell empty.
    pub fn at(path: PathBuf, tables: TaxTables) -> Self {
        if path.is_dir() {
            return Self {
                plan_path: None,
                launched: path,
                tables,
            };
        }
        let launched = directory_of(&path).to_path_buf();
        Self {
            plan_path: Some(path),
            launched,
            tables,
        }
    }

    /// The directory the pickers open on: beside the document, or the one
    /// launched on while there is none.
    pub fn workspace(&self) -> &Path {
        self.plan_path
            .as_deref()
            .map_or(&self.launched, directory_of)
    }

    /// The open document's path, or the refusal for having none.
    pub fn document(&self) -> Result<&Path, String> {
        self.plan_path
            .as_deref()
            .ok_or_else(|| NO_DOCUMENT.to_owned())
    }

    /// The plan file's own name, which is what the shell calls the plan.
    pub fn file_name(&self) -> Cow<'_, str> {
        self.plan_path
            .as_deref()
            .map_or(Cow::Borrowed(NO_DOCUMENT_NAME), file_name)
    }

    /// Whether the shell is holding no document at all.
    pub fn is_empty(&self) -> bool {
        self.plan_path.is_none()
    }
}

pub fn file_name(path: &Path) -> Cow<'_, str> {
    path.file_name()
        .map_or(Cow::Borrowed(""), |name| name.to_string_lossy())
}

impl Projected {
    /// What the empty shell holds behind its hidden pages.
    ///
    /// # Panics
    ///
    /// If the blank plan stops parsing.
    pub fn blank(tables: &TaxTables, today: Today) -> Self {
        let plan = Plan::from_toml_str(&blank_plan(today.0)).expect("the blank plan parses");
        let projection = project(&plan, tables);
        Self { plan, projection }
    }
}

/// The calendar year the session runs in.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Today(pub i16);

impl Today {
    pub fn now() -> Self {
        Self(jiff::Zoned::now().date().year())
    }
}

/// The year every view is on. Until something moves it, that is today.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct YearCursor(pub Option<i16>);

impl YearCursor {
    /// The one reading of the cursor every view shares: its own year, else
    /// today as near as the plan's years `planned` reach, then as near as
    /// the years a view `shows` reach, which are the plan's or run past them.
    pub fn resolve(self, today: Today, planned: (i16, i16), shows: (i16, i16)) -> i16 {
        let year = self.0.unwrap_or_else(|| clamp(today.0, planned));
        clamp(year, shows)
    }
}

fn clamp(year: i16, (first, last): (i16, i16)) -> i16 {
    year.max(first).min(last)
}

/// The first and last of `years`, which run unbroken; any year at all
/// where there are none.
pub fn span(years: &[YearRow]) -> (i16, i16) {
    match (years.first(), years.last()) {
        (Some(first), Some(last)) => (first.year, last.year),
        _ => (i16::MIN, i16::MAX),
    }
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

/// The cursor's year among `projected`'s years.
pub fn cursor_year(projected: &Projected, today: Today, cursor: YearCursor) -> i16 {
    let planned = span(&projected.projection.years);
    cursor.resolve(today, planned, planned)
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

    /// The Ledger's row for the cursor's year.
    pub fn row(&self) -> Option<&YearRow> {
        let year = self.year();
        let years = &self.ledger().projection.years;
        years.iter().find(|row| row.year == year)
    }
}

/// The displayed dollar basis.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct Basis {
    /// Show future (nominal) dollars instead of today's dollars.
    pub nominal: bool,
}
