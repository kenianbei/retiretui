//! What is open: the document, where it came from, the tables and store it
//! is read through, its projection, the year the session runs in, and the
//! year every view is on.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{Projection, YearRow, project};

use crate::files::{Invalid, directory_of};
use crate::store::{Stamp, Store};

/// Where the shown plan came from and what reloads project against.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::prelude::Resource))]
pub struct Session {
    /// The open plan or scenario; none in the empty shell.
    pub plan_path: Option<PathBuf>,
    /// The directory launched on, or the launched file's.
    launched: PathBuf,
    /// The tax tables loaded at launch.
    pub tables: TaxTables,
    /// Where the plan files are kept.
    pub store: Arc<dyn Store>,
    /// The directory no picker climbs above; none where any may be reached.
    pub floor: Option<PathBuf>,
}

/// Why a command needing a document refuses in the empty shell.
pub const NO_DOCUMENT: &str = "no document is open";

const NO_DOCUMENT_NAME: &str = "no document";

/// The account a household starts with, which is also where unspent
/// income sweeps.
pub const CASH_ID: &str = "cash";

/// The age every starting plan runs to.
pub const HORIZON_AGE: u8 = 95;

/// The inflation every starting plan assumes.
pub const INFLATION: f64 = 0.025;

/// The smallest plan that validates: one person and the cash account
/// surplus lands in, starting `start_year`.
#[must_use]
pub fn blank_plan(start_year: i16) -> String {
    format!(
        r#"schema = 1

[plan]
start_year = {start_year}
horizon_age = {HORIZON_AGE}
inflation = {INFLATION}

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1970-01-01

[[accounts]]
id = "{CASH_ID}"
kind = "cash"
owner = "me"
balance = 0
"#
    )
}

/// The projected plan every view derives from; replaced wholesale on reload.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::prelude::Resource))]
#[derive(Clone, Debug)]
pub struct Projected {
    /// The resolved plan.
    pub plan: Plan,
    /// Its projection.
    pub projection: Projection,
}

impl Session {
    /// A session at `path`: a plan or scenario file, or a directory, which
    /// opens the shell empty.
    pub fn at(store: Arc<dyn Store>, path: PathBuf, tables: TaxTables) -> Self {
        if store.is_dir(&path) {
            return Self {
                plan_path: None,
                launched: path,
                tables,
                store,
                floor: None,
            };
        }
        let launched = directory_of(&path).to_path_buf();
        Self {
            plan_path: Some(path),
            launched,
            tables,
            store,
            floor: None,
        }
    }

    /// The directory the pickers open on: beside the document, or the one
    /// launched on while there is none.
    pub fn workspace(&self) -> &Path {
        self.plan_path
            .as_deref()
            .map_or(&self.launched, directory_of)
    }

    /// The open document's path.
    ///
    /// # Errors
    ///
    /// The refusal to say where no document is open.
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
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.plan_path.is_none()
    }
}

/// The file name of `path`, or nothing where it has none.
#[must_use]
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
    #[must_use]
    pub fn blank(tables: &TaxTables, today: Today) -> Self {
        let plan = Plan::from_toml_str(&blank_plan(today.0)).expect("the blank plan parses");
        let projection = project(&plan, tables);
        Self { plan, projection }
    }
}

/// The calendar year the session runs in.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::prelude::Resource))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Today(pub i16);

impl Today {
    /// The year the clock says it is.
    #[must_use]
    pub fn now() -> Self {
        Self(jiff::Zoned::now().date().year())
    }
}

/// The year every view is on. Until something moves it, that is today.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::prelude::Resource))]
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct YearCursor(pub Option<i16>);

impl YearCursor {
    /// The one reading of the cursor every view shares: its own year, else
    /// today as near as the plan's years `planned` reach, then as near as
    /// the years a view `shows` reach, which are the plan's or run past them.
    #[must_use]
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
#[must_use]
pub fn span(years: &[YearRow]) -> (i16, i16) {
    match (years.first(), years.last()) {
        (Some(first), Some(last)) => (first.year, last.year),
        _ => (i16::MIN, i16::MAX),
    }
}

/// The cursor's year among `projected`'s years.
#[must_use]
pub fn cursor_year(projected: &Projected, today: Today, cursor: YearCursor) -> i16 {
    let planned = span(&projected.projection.years);
    cursor.resolve(today, planned, planned)
}

/// The resolved chain's files, each with its stamp as last seen.
pub type Stamped = Vec<(PathBuf, Option<Stamp>)>;

/// What the session shows at launch: its document projected, or the blank
/// plan behind an empty shell that watches nothing.
///
/// # Errors
///
/// The headline of why the document did not pass the load-and-validate
/// gate.
pub fn load_session(session: &Session, today: Today) -> Result<(Projected, Stamped), String> {
    match &session.plan_path {
        Some(path) => {
            let (loaded, files) = load_projected(session.store.as_ref(), path, &session.tables);
            let projected = loaded.map_err(|invalid| invalid.headline().to_owned())?;
            Ok((projected, files))
        }
        None => Ok((Projected::blank(&session.tables, today), Vec::new())),
    }
}

/// Loads, validates, and projects the plan at `path`, beside the stamped
/// chain files it read - on a failure, as far as the read got, so a fix
/// anywhere in the chain as it now stands is picked up.
///
/// The error is why the plan did not pass the gate: a reader says its
/// headline, or its reason where it names the file itself.
pub fn load_projected(
    store: &dyn Store,
    path: &Path,
    tables: &TaxTables,
) -> (Result<Projected, Invalid>, Stamped) {
    let (plan, files) = crate::files::validated_plan_with_files(store, path, tables);
    let projected = plan.map(|plan| {
        let projection = project(&plan, tables);
        Projected { plan, projection }
    });
    (projected, stamp(store, files))
}

/// Each of `files` beside its stamp as `store` has it now.
pub fn stamp(store: &dyn Store, files: Vec<PathBuf>) -> Stamped {
    files
        .into_iter()
        .map(|path| {
            let stamped = store.stamp(&path);
            (path, stamped)
        })
        .collect()
}

/// Whether any of `files` has changed in `store` since it was stamped.
pub fn is_stale(store: &dyn Store, files: &[(PathBuf, Option<Stamp>)]) -> bool {
    files
        .iter()
        .any(|(path, recorded)| store.stamp(path) != *recorded)
}

/// The `year` row; an out-of-range year errors with the valid range.
///
/// # Errors
///
/// Where the plan does not reach `year`.
pub fn year_row(projection: &Projection, year: i16) -> Result<&YearRow, String> {
    projection.row(year).ok_or_else(|| {
        let first = projection.years.first().map_or(year, |row| row.year);
        let last = projection.years.last().map_or(year, |row| row.year);
        format!("{year} is outside the projection; the plan covers {first}-{last}")
    })
}
