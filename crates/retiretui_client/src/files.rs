//! Plan files: read through their scenario chains, validated, and written
//! back whole.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Issue, Plan};
use retiretui_engine::project::validate_plan;

use crate::store::Store;
use retiretui_engine::plan::resolve;

/// Why a plan file did not pass the load-and-validate gate.
pub enum Invalid {
    /// It could not be read or resolved.
    Load(String),
    /// It was read, and failed validation.
    Issues {
        /// The line the refusal is said in.
        headline: String,
        /// Every issue, in validation order.
        issues: Vec<Issue>,
    },
}

impl Invalid {
    /// The line the refusal is said in; the rest is detail.
    #[must_use]
    pub fn headline(&self) -> &str {
        match self {
            Self::Load(message) => message.lines().next().unwrap_or_default(),
            Self::Issues { headline, .. } => headline,
        }
    }

    /// What went wrong, where the file is named already: the first issue,
    /// or the headline of a file that could not be read.
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            Self::Issues { issues, .. } if !issues.is_empty() => &issues[0].message,
            _ => self.headline(),
        }
    }
}

/// The full load-and-validate gate every surface runs, beside the files
/// the resolution read, whether or not it passed.
pub fn validated_plan_with_files(
    store: &dyn Store,
    path: &Path,
    tables: &TaxTables,
) -> (Result<Plan, Invalid>, Vec<PathBuf>) {
    let mut files = Vec::new();
    let validated = load_plan_with_files(store, path, &mut files)
        .map_err(|error| Invalid::Load(error.to_string()))
        .and_then(|plan| {
            let issues = validate_plan(&plan, tables);
            if issues.is_empty() {
                return Ok(plan);
            }
            let headline = format!("{} issue(s) found in {}:", issues.len(), path.display());
            Err(Invalid::Issues { headline, issues })
        });
    (validated, files)
}

/// `target` re-expressed relative to `from_dir`, with `/` separators. Both
/// paths must already be normalized against the same root (canonicalized,
/// or both relative to one sandbox root).
#[must_use]
pub fn relative_path(from_dir: &Path, target: &Path) -> String {
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
#[must_use]
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
pub fn overlay_base(store: &dyn Store, out: &Path, plan_path: &Path) -> std::io::Result<String> {
    let out_dir = store.canonical(directory_of(out))?;
    let plan = store.canonical(plan_path)?;
    Ok(relative_path(&out_dir, &plan))
}

/// Loads a plan or scenario file, resolving `base` chains relative to each
/// referring file, and returns every file the resolution read - the document
/// and its whole base chain - for callers that watch them.
///
/// # Errors
///
/// When a file cannot be opened or read, or the chain does not resolve.
pub fn load_plan_with_files(
    store: &dyn Store,
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> anyhow::Result<Plan> {
    let start = store
        .canonical(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let mut read = |file: &Path| {
        files.push(file.to_owned());
        store
            .read(file)
            .map_err(|error| format!("failed to read {}: {error}", file.display()))
    };
    let mut locate = |referrer: &Path, base: &str| {
        let joined = referrer.parent().unwrap_or(Path::new(".")).join(base);
        store
            .canonical(&joined)
            .map_err(|error| format!("failed to open {}: {error}", joined.display()))
    };
    let text = read(&start).map_err(anyhow::Error::msg)?;
    resolve::resolve_plan(start, text, &mut read, &mut locate).map_err(anyhow::Error::msg)
}

/// Writes `text` to `path` atomically: staged beside it, then renamed over.
///
/// # Errors
///
/// When the staged file cannot be written or renamed.
pub fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let staged = path.with_extension("toml.tmp");
    fs::write(&staged, text).and_then(|()| fs::rename(&staged, path))
}

/// Writes a plan to `path` as canonical TOML - comments and layout of the
/// source file are not preserved - atomically. The caller has validated it.
///
/// # Errors
///
/// When the plan does not serialize or the file cannot be written.
pub fn write_plan(store: &dyn Store, path: &Path, plan: &Plan) -> Result<(), String> {
    let canonical = plan
        .to_toml_string()
        .map_err(|error| format!("not saved: {error}"))?;
    store
        .write(path, &canonical)
        .map_err(|error| format!("not saved: {}: {error}", path.display()))
}
