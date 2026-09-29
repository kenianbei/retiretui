//! Scenario base chains followed to the plan they resolve to.

use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use super::{Issue, Plan, PlanError, Scenario, ScenarioError};

/// Why a document did not resolve, and in which file of its chain.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolveError {
    /// The file the failure is in: the one opened, a base in its chain, or
    /// the base that could not be read.
    pub file: PathBuf,
    /// What went wrong there.
    pub cause: ResolveCause,
}

/// What went wrong in a [`ResolveError`]'s file.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolveCause {
    /// The file's own text is not TOML, or not a plan or scenario.
    Parse {
        /// The error as `toml` says it, its position and source line included.
        said: String,
        /// The error alone.
        message: String,
        /// Where in `text` it is, in bytes, where the error knows.
        span: Option<Range<usize>>,
        /// The file's text.
        text: String,
    },
    /// The plan the scenarios merge to is not a plan; its position is in a
    /// document no one wrote.
    Merged {
        /// The error as `toml` says it.
        said: String,
        /// The error alone.
        message: String,
    },
    /// The scenario's overlay does not apply.
    Invalid(Vec<Issue>),
    /// The file could not be read or found, in the reader's words.
    Unread(String),
    /// The file is its own base, somewhere down the chain.
    Cycle,
}

impl ResolveError {
    fn parse(file: PathBuf, error: &toml::de::Error, text: String) -> Self {
        let cause = ResolveCause::Parse {
            said: error.to_string(),
            message: error.message().to_owned(),
            span: error.span(),
            text,
        };
        Self { file, cause }
    }

    fn scenario(file: PathBuf, error: &ScenarioError, text: String) -> Self {
        if let ScenarioError::Parse(error) = error {
            return Self::parse(file, error, text);
        }
        let said = error.to_string();
        let message = said.clone();
        let cause = ResolveCause::Parse {
            said,
            message,
            span: None,
            text,
        };
        Self { file, cause }
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = self.file.display();
        match &self.cause {
            ResolveCause::Parse { said, .. } => write!(f, "{file}: {said}"),
            ResolveCause::Merged { said, .. } => f.write_str(said),
            ResolveCause::Invalid(issues) => {
                let listing: Vec<String> = issues.iter().map(ToString::to_string).collect();
                write!(f, "{file}: {}", listing.join("; "))
            }
            ResolveCause::Unread(reason) => f.write_str(reason),
            ResolveCause::Cycle => write!(f, "{file}: scenario base chain forms a cycle"),
        }
    }
}

impl std::error::Error for ResolveError {}

/// Resolves a plan or scenario document into a plan, following `base`
/// references and applying overlays bottom-up. `text` is the start
/// document's content (it need not exist on disk yet); `read` returns a
/// referenced document's text; `locate` turns a `base` reference into the
/// canonical path it names, relative to the referring document. Canonical
/// paths drive cycle detection, so `locate` must return the same path for
/// the same file.
///
/// # Errors
///
/// When a document does not parse, a base cannot be read or located, or
/// the chain cycles.
pub fn resolve_plan(
    start: PathBuf,
    text: String,
    read: &mut dyn FnMut(&Path) -> Result<String, String>,
    locate: &mut dyn FnMut(&Path, &str) -> Result<PathBuf, String>,
) -> Result<Plan, ResolveError> {
    let mut visited = vec![start.clone()];
    let mut current = start;
    let mut text = text;
    let mut overlays = Vec::new();
    loop {
        match Scenario::from_toml_str(&text) {
            Ok(Some(scenario)) => {
                let next = locate(&current, scenario.base()).map_err(|reason| ResolveError {
                    file: beside(&current, scenario.base()),
                    cause: ResolveCause::Unread(reason),
                })?;
                if visited.contains(&next) {
                    return Err(ResolveError {
                        file: next,
                        cause: ResolveCause::Cycle,
                    });
                }
                visited.push(next.clone());
                overlays.push((current, scenario));
                current = next;
                text = read(&current).map_err(|reason| ResolveError {
                    file: current.clone(),
                    cause: ResolveCause::Unread(reason),
                })?;
            }
            Ok(None) => break,
            Err(error) => return Err(ResolveError::scenario(current, &error, text)),
        }
    }
    if overlays.is_empty() {
        return toml::from_str(&text).map_err(|error| ResolveError::parse(current, &error, text));
    }
    merged(current, text, overlays)
}

/// The plan `overlays`, the opened file's first, make of the base `text`
/// read from `current`.
fn merged(
    current: PathBuf,
    text: String,
    overlays: Vec<(PathBuf, Scenario)>,
) -> Result<Plan, ResolveError> {
    let mut table: toml::Table = toml::from_str(&text)
        .map_err(|error| ResolveError::parse(current.clone(), &error, text))?;
    let opened = overlays.first().map_or(current, |(path, _)| path.clone());
    for (path, scenario) in overlays.into_iter().rev() {
        table = scenario.apply(table).map_err(|issues| ResolveError {
            file: path,
            cause: ResolveCause::Invalid(issues),
        })?;
    }
    Plan::from_toml_table(table).map_err(|error| ResolveError {
        file: opened,
        cause: ResolveCause::Merged {
            said: error.to_string(),
            message: first_message(&error),
        },
    })
}

/// Where a `base` reference points, beside the file naming it, for saying
/// which file could not be found.
fn beside(referrer: &Path, base: &str) -> PathBuf {
    referrer.parent().unwrap_or(Path::new("")).join(base)
}

fn first_message(error: &PlanError) -> String {
    match error {
        PlanError::Parse(error) => error.message().to_owned(),
        PlanError::Serialize(error) => error.to_string(),
    }
}
