use std::path::{Component, Path, PathBuf};

use retiretui_engine::plan::Plan;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The sandboxed plan-file store. Every operation resolves its path inside
/// the root, so containment holds by construction.
pub struct PlanStore {
    root: PathBuf,
}

#[derive(Serialize, JsonSchema)]
pub struct PlanEntry {
    /// Path relative to the served directory.
    pub path: String,
    /// The plan's display name, when the file declares one.
    pub name: Option<String>,
    /// The `base` reference when the file is a scenario overlay.
    pub base: Option<String>,
}

#[derive(Deserialize)]
struct DocumentProbe {
    base: Option<String>,
    plan: Option<NameField>,
}

#[derive(Deserialize)]
struct NameField {
    name: Option<String>,
}

impl PlanStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn read(&self, path: &str) -> Result<String, String> {
        let resolved = self.resolve_read(path)?;
        std::fs::read_to_string(&resolved).map_err(|err| format!("{path}: {err}"))
    }

    /// Loads a plan or scenario file, resolving `base` chains relative to
    /// each referring file and contained in the root.
    pub fn load_plan(&self, path: &str) -> Result<Plan, String> {
        let start = self.resolve_read(path)?;
        let text = std::fs::read_to_string(&start).map_err(|err| format!("{path}: {err}"))?;
        self.resolve_document(start, text)
    }

    /// Fully resolves a plan or scenario document against the store as if
    /// it were stored at `path`, without writing anything.
    pub fn resolve_document_at(&self, path: &str, text: &str) -> Result<Plan, String> {
        let destination = self.resolve_write(path)?;
        self.resolve_document(destination, text.to_owned())
    }

    fn resolve_document(&self, start: PathBuf, text: String) -> Result<Plan, String> {
        let mut read = |file: &Path| {
            std::fs::read_to_string(file).map_err(|err| format!("{}: {err}", file.display()))
        };
        let mut locate = |referrer: &Path, base: &str| self.resolve_base(referrer, base);
        crate::commands::resolve::resolve_plan(start, text, &mut read, &mut locate)
    }

    /// Resolves a scenario's `base` reference relative to the referring
    /// file. `..` segments are fine; containment in the root is what gates
    /// escape.
    fn resolve_base(&self, referrer: &Path, base: &str) -> Result<PathBuf, String> {
        if Path::new(base).is_absolute() {
            return Err(format!("{base}: absolute paths are not allowed"));
        }
        let joined = referrer.parent().unwrap_or(&self.root).join(base);
        let resolved = joined
            .canonicalize()
            .map_err(|err| format!("{base}: {err}"))?;
        self.ensure_under_root(&resolved, base)?;
        Ok(resolved)
    }

    /// Writes atomically: staged in the same directory, then renamed over.
    pub fn write(&self, path: &str, text: &str) -> Result<(), String> {
        let resolved = self.resolve_write(path)?;
        crate::commands::write_atomic(&resolved, text).map_err(|err| format!("{path}: {err}"))
    }

    /// Every `*.toml` file under the root, in path order.
    pub fn list(&self) -> Result<Vec<PlanEntry>, String> {
        let mut plans = Vec::new();
        collect_plans(&self.root, &self.root, &mut plans)?;
        plans.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(plans)
    }

    fn resolve_read(&self, requested: &str) -> Result<PathBuf, String> {
        let joined = join_contained(&self.root, requested)?;
        let resolved = joined
            .canonicalize()
            .map_err(|err| format!("{requested}: {err}"))?;
        self.ensure_under_root(&resolved, requested)?;
        Ok(resolved)
    }

    fn resolve_write(&self, requested: &str) -> Result<PathBuf, String> {
        let joined = join_contained(&self.root, requested)?;
        if joined.extension().is_none_or(|ext| ext != "toml") {
            return Err(format!("{requested}: plan files must end in .toml"));
        }
        let parent = joined.parent().unwrap_or(&self.root);
        let resolved_parent = parent
            .canonicalize()
            .map_err(|err| format!("{requested}: {err}"))?;
        self.ensure_under_root(&resolved_parent, requested)?;
        let file_name = joined.file_name().expect("has a .toml extension");
        Ok(resolved_parent.join(file_name))
    }

    fn ensure_under_root(&self, resolved: &Path, requested: &str) -> Result<(), String> {
        if resolved.starts_with(&self.root) {
            Ok(())
        } else {
            Err(format!("{requested}: path escapes the served directory"))
        }
    }
}

fn join_contained(root: &Path, requested: &str) -> Result<PathBuf, String> {
    let path = Path::new(requested);
    if path.is_absolute() {
        return Err(format!("{requested}: absolute paths are not allowed"));
    }
    if !path
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err(format!("{requested}: path escapes the served directory"));
    }
    Ok(root.join(path))
}

fn collect_plans(root: &Path, dir: &Path, plans: &mut Vec<PlanEntry>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("{}: {err}", dir.display()))?;
        let path = entry.path();
        let hidden = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with('.'));
        if hidden {
            continue;
        }
        let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if is_dir {
            collect_plans(root, &path, plans)?;
        } else if path.extension().is_some_and(|ext| ext == "toml") {
            plans.push(plan_entry(root, &path));
        }
    }
    Ok(())
}

fn plan_entry(root: &Path, path: &Path) -> PlanEntry {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let probe = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str::<DocumentProbe>(&text).ok());
    PlanEntry {
        path: relative,
        name: probe
            .as_ref()
            .and_then(|probe| probe.plan.as_ref())
            .and_then(|plan| plan.name.clone()),
        base: probe.and_then(|probe| probe.base),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> PlanStore {
        PlanStore::new(std::env::temp_dir().canonicalize().expect("temp dir"))
    }

    #[test]
    fn rejects_absolute_paths() {
        let error = store().read("/etc/passwd").unwrap_err();
        assert!(error.contains("absolute"));
    }

    #[test]
    fn rejects_parent_traversal() {
        let error = store().write("../escape.toml", "").unwrap_err();
        assert!(error.contains("escapes"));
    }

    #[test]
    fn rejects_non_toml_writes() {
        let error = store().write("plan.json", "").unwrap_err();
        assert!(error.contains(".toml"));
    }

    #[test]
    fn writes_and_reads_back_in_root() {
        let store = store();
        store
            .write("store-test.toml", "schema = 1\n")
            .expect("contained");
        assert_eq!(store.read("store-test.toml").unwrap(), "schema = 1\n");
    }
}
