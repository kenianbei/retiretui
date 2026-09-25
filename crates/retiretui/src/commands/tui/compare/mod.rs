//! The Compare page: the document beside other workspace files, a row of
//! figures each in the Plans table over their metrics year by year.

mod by_year;
mod changes;
mod follow;
pub(super) mod plans;
#[cfg(test)]
mod tests;
mod views;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{
    Commands, Entity, IntoScheduleConfigs, Query, Res, ResMut, Resource, With,
};
use bevy_ecs::system::SystemParam;
use plurimus::core::UiWidget;
use plurimus::widgets::WidgetSystems;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::Projection;

use crate::commands::compare::Metric;
use crate::commands::table::basis_amount;

use super::chart::{Mark, Series, SeriesChart};
use super::command::Outcome;
use super::hints::Hints;
use super::journal;
use super::layout::{self, Body};
use super::nav::{self, Page};
use super::present::{compact_money, signed_money};
use super::session::{self, Session, Shown};
use super::success::{self, Successes};
use super::theme::{Repainted, Theme};
use super::tools::{self, HelpLine};
use super::watch;

use follow::ComparedDoc;
pub(crate) use follow::open_highlighted;
pub(crate) use views::{Charted, cycle_view, next_metric, previous_metric};

pub fn plugin(app: &mut App) {
    app.init_resource::<Compared>();
    app.init_resource::<Charted>();
    app.add_systems(Startup, spawn_compare.after(layout::spawn_frame));
    app.add_systems(
        Update,
        follow::follow_disk
            .after(watch::poll_watch)
            .run_if(watch::on_beat),
    );
    app.add_systems(
        Update,
        (
            (
                views::refresh_views,
                session::track_cursor::<by_year::ByYearTable>,
            )
                .chain(),
            show_help,
            (plans::refresh_plans, changes::refresh_changes)
                .chain()
                .after(success::work_through)
                .before(Repainted)
                .before(WidgetSystems::Layout),
        ),
    );
}

/// Walking the metrics is the page's, from whichever pane holds the keys.
const METRIC_HINTS: Hints = Hints(&[("←→", "metric")]);
const HELP: &str = "Compare this plan with others in its folder: c adds one.";
/// A plan's figure in a year its projection does not reach.
const UNREACHED: &str = "-";

/// The files compared with the document, projected as they were read,
/// and what the page measures them against.
#[derive(Resource, Default)]
pub struct Compared {
    docs: Vec<ComparedDoc>,
    /// The compared file the others are measured against; the document
    /// where there is none.
    baseline: Option<PathBuf>,
    /// Whether every plan but the baseline reads as its difference from it.
    is_difference: bool,
}

impl Compared {
    #[cfg(test)]
    pub fn has(&self, path: &Path) -> bool {
        self.paths().any(|held| held == path)
    }

    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.docs.iter().map(|doc| doc.path.as_path())
    }

    pub(crate) fn plans(&self) -> impl Iterator<Item = &Plan> {
        self.docs.iter().map(|doc| &doc.projected.plan)
    }

    /// Adds `path`, or takes it out where it is compared already.
    pub fn toggle(&mut self, path: PathBuf, tables: &TaxTables) {
        if let Some(at) = self.docs.iter().position(|doc| doc.path == path) {
            self.remove(at);
            return;
        }
        let name = session::file_name(&path).into_owned();
        match self.take_in(path, tables) {
            Ok(()) => journal::say(format!("compared {name}")),
            Err(reason) => journal::warn(format!("{name} not compared: {reason}")),
        }
    }

    /// Stops comparing the file at `at` among those compared; the
    /// document is the baseline again where it was.
    fn remove(&mut self, at: usize) {
        let doc = self.docs.remove(at);
        if self.baseline.as_ref() == Some(&doc.path) {
            self.baseline = None;
        }
        journal::say(format!(
            "{} no longer compared",
            session::file_name(&doc.path)
        ));
    }

    /// Compares `path` as it reads now, in place of what was compared
    /// under it.
    ///
    /// # Errors
    ///
    /// The first line of what the load failed on.
    pub fn take_in(&mut self, path: PathBuf, tables: &TaxTables) -> Result<(), String> {
        let doc = ComparedDoc::read(path, tables)?;
        match self.docs.iter().position(|held| held.path == doc.path) {
            Some(at) => self.docs[at] = doc,
            None => self.docs.push(doc),
        }
        Ok(())
    }

    /// The baseline's place among the plans: the document is 0.
    fn baseline_at(&self) -> usize {
        let path = self.baseline.as_ref();
        let at = path.and_then(|path| self.docs.iter().position(|doc| &doc.path == path));
        at.map_or(0, |at| at + 1)
    }
}

/// The page: the Plans row over the view pane, over a line of help.
fn spawn_compare(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
        return;
    };
    let view = nav::spawn_surface(&mut commands, body, Some(Page::Compare));
    commands.entity(view).insert(METRIC_HINTS);
    plans::spawn(&mut commands, view);
    views::spawn(&mut commands, view);
    tools::spawn_help(&mut commands, view, Page::Compare);
}

/// Every plan the page shows, and what makes them stale.
#[derive(SystemParam)]
pub(crate) struct Plans<'w> {
    shown: Shown<'w>,
    session: Res<'w, Session>,
    compared: Res<'w, Compared>,
    charted: Res<'w, Charted>,
    successes: Res<'w, Successes>,
    theme: Res<'w, Theme>,
}

impl Plans<'_> {
    /// Whether a plan, or the words and colours they are shown in, moved.
    fn is_plan_changed(&self) -> bool {
        self.shown.projected.is_changed()
            || self.session.is_changed()
            || self.compared.is_changed()
            || self.theme.is_changed()
    }

    /// Whether anything the views show moved: a plan, the basis, the year
    /// or what is charted.
    fn is_changed(&self) -> bool {
        self.is_plan_changed() || self.shown.is_changed() || self.charted.is_changed()
    }

    /// The cursor's year among every year a plan reaches.
    fn year(&self) -> i16 {
        let spans = self
            .each()
            .map(|(_, projection)| session::span(&projection.years));
        let shows = spans.fold((i16::MAX, i16::MIN), |(first, last), (from, to)| {
            (first.min(from), last.max(to))
        });
        self.shown.year_among(shows)
    }

    /// The document first, then every compared file.
    fn each(&self) -> impl Iterator<Item = (Cow<'_, str>, &Projection)> {
        let document = (self.session.file_name(), &self.shown.projected.projection);
        let compared = self.compared.docs.iter();
        std::iter::once(document)
            .chain(compared.map(|doc| (session::file_name(&doc.path), &doc.projected.projection)))
    }

    /// Each plan, in the order of [`Self::each`].
    fn each_plan(&self) -> impl Iterator<Item = &Plan> {
        let document = std::iter::once(&self.shown.projected.plan);
        document.chain(self.compared.plans())
    }

    /// The plan the others are measured against, where they read as
    /// differences: its place, name and projection.
    fn against(&self) -> Option<(usize, Cow<'_, str>, &Projection)> {
        if !self.compared.is_difference {
            return None;
        }
        let at = self.compared.baseline_at();
        let (name, projection) = self.each().nth(at)?;
        Some((at, name, projection))
    }

    /// Each plan's `metric` year by year on the basis shown; under
    /// difference, each but the baseline less the baseline's, in the years
    /// both reach.
    fn amounts(&self, metric: Metric) -> Vec<BTreeMap<i16, Dollars>> {
        let nominal = self.shown.basis.nominal;
        let own: Vec<BTreeMap<i16, Dollars>> = (self.each())
            .map(|(_, projection)| {
                let rows = projection.years.iter();
                rows.map(|row| {
                    let amount = basis_amount(metric.value(row), row.deflator, nominal);
                    (row.year, amount)
                })
                .collect()
            })
            .collect();
        let Some((at, ..)) = self.against() else {
            return own;
        };
        let base = &own[at];
        let less_base = |amounts: &BTreeMap<i16, Dollars>| {
            let each = amounts.iter();
            each.filter_map(|(year, amount)| Some((*year, amount - base.get(year)?)))
                .collect()
        };
        (own.iter().enumerate())
            .map(|(place, amounts)| {
                if place == at {
                    amounts.clone()
                } else {
                    less_base(amounts)
                }
            })
            .collect()
    }

    /// A line per plan; under difference, each less the baseline's, which
    /// is drawn along zero in its own colour, the Plans table being the
    /// legend.
    fn chart(&self, metric: Metric) -> SeriesChart {
        let baseline = self.against().map(|(at, ..)| at);
        let series = (self.each().zip(self.amounts(metric)).enumerate())
            .map(|(place, ((label, _), amounts))| {
                let is_baseline = baseline == Some(place);
                let points = (amounts.into_iter())
                    .map(|(year, amount)| {
                        let drawn = if is_baseline { 0.0 } else { amount as f64 };
                        (f64::from(year), drawn)
                    })
                    .collect();
                let color = self.theme.series(place);
                Series {
                    label: label.into_owned(),
                    color,
                    points,
                }
            })
            .collect();
        let mut chart = SeriesChart::of(series, self.theme.dimmed());
        chart.is_legend_hidden = true;
        chart.marks = vec![Mark::cursor(self.year(), &self.theme)];
        chart
    }

    /// Each plan's figure in `year` of its [`Self::amounts`] - signed
    /// where it is a difference from the baseline - a dash where the plan
    /// does not reach it.
    fn figures_in(&self, amounts: &[BTreeMap<i16, Dollars>], year: i16) -> Vec<String> {
        let baseline = self.against().map(|(at, ..)| at);
        (amounts.iter().enumerate())
            .map(|(place, amounts)| {
                let amount = amounts.get(&year).copied();
                let is_difference = baseline.is_some_and(|at| at != place);
                let figure = if is_difference {
                    amount.map(signed_money)
                } else {
                    amount.map(compact_money)
                };
                figure.unwrap_or_else(|| UNREACHED.to_owned())
            })
            .collect()
    }
}

fn show_help(theme: Res<Theme>, mut lines: Query<(&mut UiWidget, &HelpLine)>) {
    if theme.is_changed() {
        tools::show_help(&mut lines, Page::Compare, HELP, &theme);
    }
}

/// Stops comparing the highlighted plan; the document is not one.
pub(crate) fn remove_highlighted(
    cursor: plans::Cursor,
    session: Res<Session>,
    mut compared: ResMut<Compared>,
) -> Outcome {
    match cursor.place().checked_sub(1) {
        Some(at) if at < compared.docs.len() => {
            compared.remove(at);
            Outcome::Done
        }
        _ => Outcome::Refused(format!(
            "{} is the document, not a compared plan",
            session.file_name()
        )),
    }
}

/// Measures the other plans against the highlighted one.
pub(crate) fn choose_baseline(cursor: plans::Cursor, mut compared: ResMut<Compared>) -> Outcome {
    let at = cursor.place().checked_sub(1);
    let doc = at.and_then(|at| compared.docs.get(at));
    compared.baseline = doc.map(|doc| doc.path.clone());
    Outcome::Done
}

/// Turns every plan but the baseline to its difference from it, or back.
pub(crate) fn toggle_difference(mut compared: ResMut<Compared>) -> Outcome {
    compared.is_difference = !compared.is_difference;
    Outcome::Done
}
