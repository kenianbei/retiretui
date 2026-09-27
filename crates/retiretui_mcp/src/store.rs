use std::path::{Component, Path, PathBuf};

use retiretui_client::store::Store;
use retiretui_engine::plan::Plan;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The sandboxed plan-file store. Every operation resolves its path inside
/// the root, so containment holds by construction, and reaches the files
/// through `files`.
pub struct PlanStore {
    root: PathBuf,
    files: Box<dyn Store>,
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
    pub fn new(root: PathBuf, files: Box<dyn Store>) -> Self {
        Self { root, files }
    }

    pub fn read(&self, path: &str) -> Result<String, String> {
        let resolved = self.resolve_read(path)?;
        self.files
            .read(&resolved)
            .map_err(|err| format!("{path}: {err}"))
    }

    /// Loads a plan or scenario file, resolving `base` chains relative to
    /// each referring file and contained in the root.
    pub fn load_plan(&self, path: &str) -> Result<Plan, String> {
        let start = self.resolve_read(path)?;
        let text = self
            .files
            .read(&start)
            .map_err(|err| format!("{path}: {err}"))?;
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
            self.files
                .read(file)
                .map_err(|err| format!("{}: {err}", file.display()))
        };
        let mut locate = |referrer: &Path, base: &str| self.resolve_base(referrer, base);
        retiretui_engine::plan::resolve::resolve_plan(start, text, &mut read, &mut locate)
    }

    /// Resolves a scenario's `base` reference relative to the referring
    /// file. `..` segments are fine; containment in the root is what gates
    /// escape.
    fn resolve_base(&self, referrer: &Path, base: &str) -> Result<PathBuf, String> {
        if Path::new(base).is_absolute() {
            return Err(format!("{base}: absolute paths are not allowed"));
        }
        let joined = referrer.parent().unwrap_or(&self.root).join(base);
        let resolved = self
            .files
            .canonical(&joined)
            .map_err(|err| format!("{base}: {err}"))?;
        self.ensure_under_root(&resolved, base)?;
        Ok(resolved)
    }

    /// Writes atomically: staged in the same directory, then renamed over.
    pub fn write(&self, path: &str, text: &str) -> Result<(), String> {
        let resolved = self.resolve_write(path)?;
        self.files
            .write(&resolved, text)
            .map_err(|err| format!("{path}: {err}"))
    }

    /// Every `*.toml` file under the root, in path order.
    pub fn list(&self) -> Result<Vec<PlanEntry>, String> {
        let mut plans = Vec::new();
        self.collect_plans(&self.root, &mut plans)?;
        plans.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(plans)
    }

    fn resolve_read(&self, requested: &str) -> Result<PathBuf, String> {
        let joined = join_contained(&self.root, requested)?;
        let resolved = self
            .files
            .canonical(&joined)
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
        let resolved_parent = self
            .files
            .canonical(parent)
            .map_err(|err| format!("{requested}: {err}"))?;
        self.ensure_under_root(&resolved_parent, requested)?;
        let file_name = joined.file_name().expect("has a .toml extension");
        Ok(resolved_parent.join(file_name))
    }

    /// Every `*.toml` file in `dir` and the directories beneath it, save
    /// hidden entries. A linked directory is not entered, so a listing
    /// cannot leave the root or loop.
    fn collect_plans(&self, dir: &Path, plans: &mut Vec<PlanEntry>) -> Result<(), String> {
        let entries = self
            .files
            .list(dir)
            .map_err(|err| format!("{}: {err}", dir.display()))?;
        for entry in entries
            .into_iter()
            .filter(|entry| !entry.name.starts_with('.'))
        {
            let path = dir.join(&entry.name);
            if entry.is_dir && self.files.canonical(&path).is_ok_and(|real| real == path) {
                self.collect_plans(&path, plans)?;
            } else if path.extension().is_some_and(|ext| ext == "toml") {
                plans.push(self.plan_entry(&path));
            }
        }
        Ok(())
    }

    fn plan_entry(&self, path: &Path) -> PlanEntry {
        let relative = path
            .strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let probe = self
            .files
            .read(path)
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

#[cfg(test)]
mod tests {
    use super::*;

    use retiretui_client::store::DiskStore;

    fn store() -> PlanStore {
        let root = std::env::temp_dir().canonicalize().expect("temp dir");
        PlanStore::new(root, Box::new(DiskStore))
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

    #[cfg(unix)]
    #[test]
    fn lists_no_plan_through_a_linked_directory() {
        let root = std::env::temp_dir().join("retiretui-store-links");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("inner")).unwrap();
        std::fs::write(root.join("inner/plan.toml"), "schema = 1\n").unwrap();
        std::os::unix::fs::symlink(root.join("inner"), root.join("linked")).unwrap();
        std::os::unix::fs::symlink(&root, root.join("inner/loop")).unwrap();
        let store = PlanStore::new(root.canonicalize().unwrap(), Box::new(DiskStore));
        let paths: Vec<String> = store
            .list()
            .unwrap()
            .into_iter()
            .map(|entry| entry.path)
            .collect();
        assert_eq!(paths, ["inner/plan.toml"]);
    }
}
