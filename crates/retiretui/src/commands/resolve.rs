use std::path::{Path, PathBuf};

use retiretui_engine::plan::{Plan, Scenario};

/// Resolves a plan or scenario document into a plan, following `base`
/// references and applying overlays bottom-up. `text` is the start
/// document's content (it need not exist on disk yet); `read` returns a
/// referenced document's text; `locate` turns a `base` reference into the
/// canonical path it names, relative to the referring document. Canonical
/// paths drive cycle detection, so `locate` must return the same path for
/// the same file.
pub fn resolve_plan(
    start: PathBuf,
    text: String,
    read: &mut dyn FnMut(&Path) -> Result<String, String>,
    locate: &mut dyn FnMut(&Path, &str) -> Result<PathBuf, String>,
) -> Result<Plan, String> {
    let mut visited = vec![start.clone()];
    let mut current = start;
    let mut text = text;
    let mut overlays = Vec::new();
    loop {
        match Scenario::from_toml_str(&text) {
            Ok(Some(scenario)) => {
                let next = locate(&current, scenario.base())?;
                if visited.contains(&next) {
                    return Err(format!(
                        "{}: scenario base chain forms a cycle",
                        next.display()
                    ));
                }
                visited.push(next.clone());
                overlays.push((current, scenario));
                current = next;
                text = read(&current)?;
            }
            Ok(None) => break,
            Err(error) => return Err(format!("{}: {error}", current.display())),
        }
    }
    if overlays.is_empty() {
        return Plan::from_toml_str(&text)
            .map_err(|error| format!("{}: {error}", current.display()));
    }
    let mut table: toml::Table =
        toml::from_str(&text).map_err(|error| format!("{}: {error}", current.display()))?;
    for (path, scenario) in overlays.into_iter().rev() {
        table = scenario.apply(table).map_err(|issues| {
            let listing: Vec<String> = issues.iter().map(ToString::to_string).collect();
            format!("{}: {}", path.display(), listing.join("; "))
        })?;
    }
    Plan::from_toml_table(table).map_err(|error| error.to_string())
}
