//! A scenario's session: edited as any draft, saved into its own overlay,
//! refusing what no overlay can state.

use std::fs;
use std::path::{Path, PathBuf};

use plurimus::term::KeyCode;
use retiretui_client::files::load_plan_with_files;
use retiretui_client::store::DiskStore;
use retiretui_engine::plan::Scenario;

use super::{ADD_ACCOUNT, Draft, clear_field, draft_plan, is_editing, open, tab_to_field};
use crate::nav::Page;
use crate::support::{
    self, SIZE, composed_frame, headless_app_at, press_ctrl, press_key, run_command, said,
    type_text,
};

const HORIZON_FIELD: usize = 2;

/// The scratch scenario and the plan it names beneath it.
fn scenario_and_base() -> (PathBuf, PathBuf) {
    let scenario = support::scratch_scenario();
    let base = scenario.with_extension("").with_extension("toml");
    (scenario, base)
}

fn resolved(path: &Path) -> retiretui_engine::plan::Plan {
    load_plan_with_files(&DiskStore, path, &mut Vec::new()).unwrap()
}

#[test]
fn an_edit_is_saved_into_the_scenarios_own_overlay_over_its_base_as_it_is_now() {
    let (scenario, base) = scenario_and_base();
    let mut app = headless_app_at(scenario.clone(), SIZE);
    open(&mut app, Page::Settings);
    tab_to_field(&mut app, HORIZON_FIELD);
    clear_field(&mut app);
    type_text(&mut app, "80");
    press_key(&mut app, KeyCode::Enter);
    let moved = fs::read_to_string(&base).unwrap().replace("0.025", "0.03");
    fs::write(&base, &moved).unwrap();
    press_ctrl(&mut app, KeyCode::Char('s'));
    let text = fs::read_to_string(&scenario).unwrap();
    let saved = Scenario::from_toml_str(&text)
        .unwrap()
        .expect("still a scenario");
    assert_eq!(
        Some(saved.base()),
        base.file_name().and_then(|name| name.to_str())
    );
    for stated in [
        "name = \"variant\"",
        "horizon_age = 80",
        "inflation = 0.025",
    ] {
        assert!(text.contains(stated), "{stated} in {text}");
    }
    assert_eq!(
        resolved(&scenario),
        draft_plan(&app),
        "resolves to the draft"
    );
    assert_eq!(
        fs::read_to_string(&base).unwrap(),
        moved,
        "the base untouched"
    );
    assert!(!app.world().resource::<Draft>().is_dirty());
}

#[test]
fn clearing_what_the_base_states_is_refused_in_the_forms_words() {
    let (scenario, base) = scenario_and_base();
    let mut app = headless_app_at(scenario, SIZE);
    open(&mut app, Page::Settings);
    clear_field(&mut app);
    press_key(&mut app, KeyCode::Enter);
    let refused = said(&app).last().cloned().unwrap_or_default();
    let base = base.file_name().unwrap().to_string_lossy();
    assert!(
        refused.ends_with(&format!(
            "Plan name: a scenario cannot clear what its base states; clear it in {base}"
        )),
        "{refused}"
    );
    let name = draft_plan(&app).plan.name;
    assert_eq!(name.as_deref(), Some("variant"), "the draft left as it was");
}

#[test]
fn a_reload_reads_again_what_is_beneath_the_scenario() {
    let (scenario, base) = scenario_and_base();
    let mut app = headless_app_at(scenario, SIZE);
    let unnamed = fs::read_to_string(&base)
        .unwrap()
        .replace("name = \"test-plan\"\n", "");
    fs::write(&base, unnamed).unwrap();
    run_command(&mut app, "reload");
    open(&mut app, Page::Settings);
    clear_field(&mut app);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(
        draft_plan(&app).plan.name,
        None,
        "the base names nothing now"
    );
}

#[test]
fn a_read_only_session_refuses_to_edit() {
    let mut app = headless_app_at(support::scratch_scenario(), SIZE);
    let plan = draft_plan(&app);
    app.world_mut().insert_resource(Draft::new(plan, true));
    open(&mut app, Page::Accounts);
    assert!(!is_editing(&app));
    let frame = composed_frame(&app);
    assert!(frame.contains("read-only"), "{frame}");
    assert!(
        !frame.contains(ADD_ACCOUNT),
        "nothing offers what the session refuses: {frame}"
    );
}
