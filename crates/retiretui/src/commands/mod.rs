pub mod actions;
pub mod compare;
pub mod import;
pub mod markets;
pub mod mcp;
pub mod optimize;
pub mod project;
mod resolve;
mod table;
pub mod tui;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Issue, Plan};
use retiretui_engine::project::validate_plan;

pub fn run_validate(path: &Path) -> anyhow::Result<()> {
    let tables = load_tables(&[])?;
    load_validated_plan(path, &tables)?;
    println!("{}: ok", path.display());
    Ok(())
}

/// Loads a plan or scenario file and refuses an invalid one, printing its
/// issues to stderr.
fn load_validated_plan(path: &Path, tables: &TaxTables) -> anyhow::Result<Plan> {
    let invalid = match validated_plan_with_files(path, tables) {
        Ok((plan, _)) => return Ok(plan),
        Err(invalid) => invalid,
    };
    match &invalid {
        Invalid::Load(message) => message.lines().skip(1).for_each(|line| eprintln!("{line}")),
        Invalid::Issues { issues, .. } => issues.iter().for_each(|issue| eprintln!("{issue}")),
    }
    Err(anyhow::Error::msg(invalid.headline().to_owned()))
}

/// Why a plan file did not pass the load-and-validate gate.
pub(crate) enum Invalid {
    /// It could not be read or resolved.
    Load(String),
    /// It was read, and failed validation.
    Issues {
        headline: String,
        issues: Vec<Issue>,
    },
}

impl Invalid {
    /// The line the refusal is said in; the rest is detail.
    pub(crate) fn headline(&self) -> &str {
        match self {
            Self::Load(message) => message.lines().next().unwrap_or_default(),
            Self::Issues { headline, .. } => headline,
        }
    }

    /// What went wrong, where the file is named already: the first issue,
    /// or the headline of a file that could not be read.
    pub(crate) fn reason(&self) -> &str {
        match self {
            Self::Issues { issues, .. } if !issues.is_empty() => &issues[0].message,
            _ => self.headline(),
        }
    }
}

/// The full load-and-validate gate every surface runs, also returning the
/// files the resolution read.
fn validated_plan_with_files(
    path: &Path,
    tables: &TaxTables,
) -> Result<(Plan, Vec<PathBuf>), Invalid> {
    let (plan, files) =
        load_plan_with_files(path).map_err(|error| Invalid::Load(error.to_string()))?;
    let issues = validate_plan(&plan, tables);
    if issues.is_empty() {
        return Ok((plan, files));
    }
    let headline = format!("{} issue(s) found in {}:", issues.len(), path.display());
    Err(Invalid::Issues { headline, issues })
}

/// One issue per line, in validation order.
fn issue_listing(issues: &[Issue]) -> String {
    let listing: Vec<String> = issues.iter().map(ToString::to_string).collect();
    listing.join("\n")
}

/// `target` re-expressed relative to `from_dir`, with `/` separators. Both
/// paths must already be normalized against the same root (canonicalized,
/// or both relative to one sandbox root).
pub(crate) fn relative_path(from_dir: &Path, target: &Path) -> String {
    let shared = target
        .components()
        .zip(from_dir.components())
        .take_while(|(target, from)| target == from)
        .count();
    let mut parts: Vec<String> = Vec::new();
    for _ in from_dir.components().skip(shared) {
        parts.push("..".to_owned());
    }
    for component in target.components().skip(shared) {
        parts.push(component.as_os_str().to_string_lossy().into_owned());
    }
    parts.join("/")
}

/// The directory `path` is in: its parent, or the working directory for a
/// bare name.
pub(crate) fn directory_of(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

/// The `base` an overlay written at `out` names `plan_path` by: relative
/// to the directory it is written into.
///
/// # Errors
///
/// When either path cannot be canonicalized.
pub(crate) fn overlay_base(out: &Path, plan_path: &Path) -> std::io::Result<String> {
    let out_dir = directory_of(out).canonicalize()?;
    let plan = plan_path.canonicalize()?;
    Ok(relative_path(&out_dir, &plan))
}

/// Loads a plan or scenario file, resolving `base` chains relative to each
/// referring file, and returns every file the resolution read - the document
/// and its whole base chain - for callers that watch them.
fn load_plan_with_files(path: &Path) -> anyhow::Result<(Plan, Vec<PathBuf>)> {
    let start = path
        .canonicalize()
        .with_context(|| format!("failed to open {}", path.display()))?;
    let mut files = Vec::new();
    let mut read = |file: &Path| {
        files.push(file.to_owned());
        fs::read_to_string(file)
            .map_err(|error| format!("failed to read {}: {error}", file.display()))
    };
    let mut locate = |referrer: &Path, base: &str| {
        let joined = referrer.parent().unwrap_or(Path::new(".")).join(base);
        joined
            .canonicalize()
            .map_err(|error| format!("failed to open {}: {error}", joined.display()))
    };
    let text = read(&start).map_err(anyhow::Error::msg)?;
    let plan =
        resolve::resolve_plan(start, text, &mut read, &mut locate).map_err(anyhow::Error::msg)?;
    Ok((plan, files))
}

/// Writes `text` to `path` atomically: staged beside it, then renamed over.
pub(crate) fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let staged = path.with_extension("toml.tmp");
    fs::write(&staged, text).and_then(|()| fs::rename(&staged, path))
}

/// The plan `text` holds with a statement's earnings recorded on `person`.
/// A scenario is refused: a resolved plan cannot be written back into an
/// overlay, so the record belongs in its base plan.
pub(crate) fn adopt_statement(text: &str, person: &str, xml: &str) -> Result<Plan, String> {
    use retiretui_engine::plan::Scenario;
    let is_scenario = Scenario::from_toml_str(text)
        .map_err(|error| error.to_string())?
        .is_some();
    if is_scenario {
        return Err("a scenario; import the record into its base plan".to_owned());
    }
    let mut plan = Plan::from_toml_str(text).map_err(|error| error.to_string())?;
    let statement = retiretui_engine::statement::parse(xml).map_err(|error| error.to_string())?;
    plan.adopt_earnings(person, &statement)
        .map_err(|issue| issue.message)?;
    Ok(plan)
}

/// Writes a plan to `path` as canonical TOML - comments and layout of the
/// source file are not preserved - atomically. The caller has validated it.
pub(crate) fn write_plan(path: &Path, plan: &Plan) -> Result<(), String> {
    let canonical = plan
        .to_toml_string()
        .map_err(|error| format!("not saved: {error}"))?;
    write_atomic(path, &canonical)
        .map_err(|error| format!("not saved: {}: {error}", path.display()))
}

/// The user's own directory `name` in the application's config directory.
pub(crate) fn user_config_dir(name: &str) -> Option<PathBuf> {
    use etcetera::BaseStrategy;
    let strategy = etcetera::choose_base_strategy().ok()?;
    Some(strategy.config_dir().join("retiretui").join(name))
}

fn load_tables(extra_dirs: &[PathBuf]) -> anyhow::Result<TaxTables> {
    let mut tables = TaxTables::embedded();
    if let Some(dir) = user_config_dir("tax") {
        tables.add_dir(&dir)?;
    }
    for dir in extra_dirs {
        tables.add_dir(dir)?;
    }
    Ok(tables)
}
