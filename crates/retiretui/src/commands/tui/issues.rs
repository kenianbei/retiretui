//! The issues panel: every issue the draft has, drawn up from the bottom
//! of the body, each row a jump to what it is about.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, ResMut, Resource, With,
};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::KeyboardInput;
use bevy_input_focus::FocusedInput;
use plurimus::core::ratatui_core::text::Line;
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{ModalDismiss, ValueChange, first_bound};
use plurimus::widgets::{ActiveDescendant, list_item};
use retiretui_engine::plan::Issue;

use super::command::Outcome;
use super::edit::{self, Draft, Turn};
use super::hints::Hints;
use super::journal;
use super::overlay::{self, Standing};
use super::theme::Theme;

pub fn plugin(app: &mut App) {
    app.init_resource::<Issues>();
    // Closes before the dialog a jump may open.
    app.add_systems(
        Update,
        (sync_issues, refresh_issues).in_set(overlay::Settles),
    );
}

const TITLE: &str = "Issues";
const NO_ISSUES: &str = "no issues";
const HINTS: Hints = Hints(&[("⏎", "go"), ("esc", "close")]);

/// Whether the panel is open.
#[derive(Resource, Default)]
pub struct Issues(bool);

#[derive(Component, Default, Debug)]
struct IssuesRoot;

#[derive(Component)]
struct IssueList;

/// Which of the draft's issues a row shows.
#[derive(Component, Clone, Copy)]
struct IssueRow(usize);

/// The `issues` command: opens the panel, or closes it.
pub fn toggle(mut issues: ResMut<Issues>) -> Outcome {
    issues.0 = !issues.0;
    Outcome::Done
}

fn sync_issues(
    issues: Res<Issues>,
    draft: Res<Draft>,
    theme: Res<Theme>,
    mut standing: Standing<IssuesRoot>,
    mut commands: Commands,
) {
    if !issues.is_changed() {
        return;
    }
    if !issues.0 {
        standing.close(&mut commands);
        return;
    }
    let Some(root) = standing.open(&mut commands) else {
        return;
    };
    let list = overlay::bottom_panel(&mut commands, root, TITLE, HINTS);
    commands
        .entity(root)
        .observe(handle_key)
        .observe(handle_dismiss);
    commands
        .entity(list)
        .insert(IssueList)
        .observe(handle_chosen);
    spawn_rows(&mut commands, list, &draft, &theme);
    standing.focus(list);
}

/// The rows follow the draft while the panel stands.
fn refresh_issues(
    draft: Res<Draft>,
    theme: Res<Theme>,
    lists: Query<Entity, With<IssueList>>,
    mut commands: Commands,
) {
    if draft.is_changed()
        && let Ok(list) = lists.single()
    {
        commands.entity(list).despawn_related::<Children>();
        spawn_rows(&mut commands, list, &draft, &theme);
    }
}

/// One row per issue, the cursor on the first.
fn spawn_rows(commands: &mut Commands, list: Entity, draft: &Draft, theme: &Theme) {
    let mut first = None;
    for (index, issue) in draft.issues().iter().enumerate() {
        let line = Line::styled(edit::issue_words(issue, draft), theme.exceeded());
        let row = commands
            .spawn((IssueRow(index), list_item(line), ChildOf(list)))
            .id();
        first.get_or_insert(row);
    }
    if first.is_none() {
        let line = Line::styled(NO_ISSUES, theme.dimmed());
        commands.spawn((list_item(line), ChildOf(list)));
    }
    commands.entity(list).insert(ActiveDescendant(first));
}

/// What a jump reads and turns.
#[derive(SystemParam)]
struct Jump<'w, 's> {
    rows: Query<'w, 's, &'static IssueRow>,
    draft: Res<'w, Draft>,
    issues: ResMut<'w, Issues>,
    turn: Turn<'w, 's>,
}

impl Jump<'_, '_> {
    fn issue_of(&self, row: Entity) -> Option<&Issue> {
        self.draft.issues().get(self.rows.get(row).ok()?.0)
    }

    /// Turns to the page the issue on `row` is about and puts the cursor
    /// on its item; the turn asks about unapplied edits as any turn does.
    fn go(&mut self, row: Entity) {
        let Some(issue) = self.issue_of(row) else {
            return;
        };
        let Some((page, index)) = edit::issue_target(&issue.path) else {
            journal::warn(format!("{}: no page shows it", issue.path));
            return;
        };
        self.issues.0 = false;
        self.turn.to(page, index);
    }
}

fn handle_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    mut issues: ResMut<Issues>,
) {
    if first_bound(overlay::CLOSE_KEYS, &input.input, held.get()).is_some() {
        input.propagate(false);
        issues.0 = false;
    }
}

/// A row chosen by Enter or a click.
fn handle_chosen(chosen: On<ValueChange<Entity>>, mut jump: Jump) {
    jump.go(chosen.value);
}

fn handle_dismiss(_dismissed: On<ModalDismiss>, mut issues: ResMut<Issues>) {
    issues.0 = false;
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use plurimus::term::KeyCode;
    use plurimus::widgets::ActiveDescendant;
    use retiretui_engine::plan::Plan;

    use super::*;
    use crate::commands::tui::edit::Row;
    use crate::commands::tui::edit::tests::cursor;
    use crate::commands::tui::nav::Page;
    use crate::commands::tui::support::{
        SIZE, active_page as active, commit_edit, composed_frame, headless_app, press_key, redrawn,
    };

    fn is_open(app: &App) -> bool {
        app.world().resource::<Issues>().0
    }

    fn count(app: &App) -> usize {
        app.world().resource::<Draft>().issues().len()
    }

    fn break_the_start_year(plan: &mut Plan) {
        plan.plan.start_year = 1000;
    }

    #[test]
    fn issue_paths_map_to_pages_by_their_longest_root() {
        let cases = [
            ("accounts[2].locked_until", Some((Page::Accounts, Some(2)))),
            ("cliffs[10]", Some((Page::Cliffs, Some(10)))),
            ("household.people[0].birth", Some((Page::People, Some(0)))),
            ("household.people", Some((Page::People, None))),
            ("household.filing", Some((Page::Household, None))),
            ("medicare.prior_magi", Some((Page::Household, None))),
            ("plan.withdrawal_order", Some((Page::Settings, None))),
            ("options.sources[0]", None),
            ("accountsx[1]", None),
        ];
        for (path, target) in cases {
            assert_eq!(edit::issue_target(path), target, "{path}");
        }
        let mut plan = crate::commands::tui::support::test_projected().plan;
        plan.accounts[1].balance = -1;
        let issue = &plan.validate()[0];
        assert_eq!(
            edit::issue_target(&issue.path),
            Some((Page::Accounts, Some(1))),
            "{issue}"
        );
    }

    #[test]
    fn the_panel_lists_every_issue_and_enter_jumps_to_the_item() {
        let mut app = headless_app(SIZE);
        press_key(&mut app, KeyCode::Char('i'));
        let frame = composed_frame(&app);
        assert!(frame.contains(NO_ISSUES), "{frame}");
        press_key(&mut app, KeyCode::Esc);
        assert!(!is_open(&app));

        commit_edit(&mut app, |plan| plan.accounts[1].balance = -1);
        commit_edit(&mut app, break_the_start_year);
        let before = count(&app);
        assert!(before > 1);
        let frame = redrawn(&mut app);
        assert!(frame.contains(&format!("{before} issues")), "{frame}");
        press_key(&mut app, KeyCode::Char('i'));
        let frame = composed_frame(&app);
        assert!(frame.contains("Accounts › k › Balance: must"), "{frame}");
        assert!(frame.contains("Settings › Start year:"), "{frame}");
        assert!(frame.contains("⏎ go"), "{frame}");

        let account_row = {
            let issues = app.world().resource::<Draft>().issues();
            issues
                .iter()
                .position(|issue| issue.path.starts_with("accounts"))
                .unwrap()
        };
        for _ in 0..account_row {
            press_key(&mut app, KeyCode::Down);
        }
        press_key(&mut app, KeyCode::Enter);
        assert!(!is_open(&app), "the jump closes the panel");
        assert_eq!(active(&app), Page::Accounts);
        app.update();
        assert_eq!(cursor(&mut app), Row(1), "the cursor is on the item");

        commit_edit(&mut app, |plan| plan.accounts[1].balance = 1);
        assert_eq!(count(&app), before - 1);
        let frame = redrawn(&mut app);
        assert!(frame.contains(&format!("{} issue", before - 1)), "{frame}");
        press_key(&mut app, KeyCode::Char('i'));
        press_key(&mut app, KeyCode::Enter);
        assert_eq!(active(&app), Page::Settings, "a single domain just turns");
    }

    #[test]
    fn the_list_follows_the_draft_while_open() {
        let mut app = headless_app(SIZE);
        commit_edit(&mut app, break_the_start_year);
        press_key(&mut app, KeyCode::Char('i'));
        commit_edit(&mut app, |plan| plan.plan.start_year = 2026);
        let frame = redrawn(&mut app);
        assert!(frame.contains(NO_ISSUES), "{frame}");
        let list = {
            let mut lists = app
                .world_mut()
                .query_filtered::<&ActiveDescendant, With<IssueList>>();
            lists.single(app.world()).unwrap().0
        };
        assert!(list.is_none(), "nothing to go to");
    }
}
