//! The dialog: one question over whatever asked it, and what each answer
//! runs.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, ResMut, Resource,
};
use bevy_input::keyboard::KeyboardInput;
use bevy_input_focus::FocusedInput;
use bevy_input_focus::tab_navigation::TabGroup;
use plurimus::core::UiWidget;
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{ModalDismiss, ModalOpen, first_bound};
use plurimus::widgets::ratatui_widgets::paragraph::{Paragraph, Wrap};
use plurimus::widgets::{Activate, button};

use super::hints::Hints;
use super::layout::{Emphasis, button_node, fixed, growing, placed, spawn_button_row, wrapped};
use super::overlay::{self, Standing};
use super::pane::Framed;
use super::picker;
use super::theme::Repainted;

pub fn plugin(app: &mut App) {
    app.init_resource::<Confirm>();
    app.add_systems(Update, sync_confirm.in_set(Repainted).after(picker::Synced));
}

const TITLE: &str = "Confirm";
const WIDTH: u16 = 48;
/// What the question has of the width, inside the frame.
const TEXT_COLS: u16 = WIDTH - 2;
/// Past the question's rows: a blank one, and the one the answers sit on.
const ANSWER_ROWS: u16 = 2;
#[derive(Component, Default, Debug)]
struct ConfirmRoot;

/// The answer a button gives, by its place among those asked.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
struct Answers(usize);

/// What an answer runs: a one-shot the asker builds, so what the question
/// is about rides with the question.
type Then = Box<dyn FnOnce(&mut Commands) + Send + Sync>;

/// One way of answering: what its button says, and what choosing it runs.
pub struct Answer {
    label: &'static str,
    then: Option<Then>,
    emphasis: Option<Emphasis>,
}

impl Answer {
    #[must_use]
    pub const fn closing(label: &'static str) -> Self {
        Self {
            label,
            then: None,
            emphasis: None,
        }
    }

    #[must_use]
    pub const fn primary(mut self) -> Self {
        self.emphasis = Some(Emphasis::Primary);
        self
    }

    #[must_use]
    pub const fn destructive(mut self) -> Self {
        self.emphasis = Some(Emphasis::Destructive);
        self
    }

    #[must_use]
    pub fn running(
        label: &'static str,
        then: impl FnOnce(&mut Commands) + Send + Sync + 'static,
    ) -> Self {
        Self {
            label,
            then: Some(Box::new(then)),
            emphasis: None,
        }
    }
}

struct Asked {
    message: String,
    answers: Vec<Answer>,
}

/// The question on show, and nothing where none is asked.
#[derive(Resource, Default)]
pub struct Confirm(Option<Asked>);

impl Confirm {
    /// Asks `message`, running `then` where the answer is `yes`: the verb
    /// of what is asked, so the button says what pressing it does. The
    /// dialog stands over whatever asked it, which stays open beneath.
    pub fn ask(
        &mut self,
        message: impl Into<String>,
        yes: &'static str,
        then: impl FnOnce(&mut Commands) + Send + Sync + 'static,
    ) {
        let answers = vec![
            Answer::closing("Cancel"),
            Answer::running(yes, then).destructive(),
        ];
        self.ask_among(message, answers);
    }

    /// Asks `message` with `answers` along the dialog's foot, the keyboard
    /// opening on the last: the answer the asker came to give. Esc, or a
    /// press outside, gives none of them.
    pub fn ask_among(&mut self, message: impl Into<String>, answers: Vec<Answer>) {
        self.0 = Some(Asked {
            message: message.into(),
            answers,
        });
    }

    #[cfg(test)]
    pub const fn is_open(&self) -> bool {
        self.0.is_some()
    }
}

fn sync_confirm(
    confirm: Res<Confirm>,
    mut standing: Standing<ConfirmRoot>,
    mut commands: Commands,
) {
    if !confirm.is_changed() {
        return;
    }
    let Some(asked) = confirm.0.as_ref() else {
        standing.close(&mut commands);
        return;
    };
    let Some(root) = standing.open(&mut commands) else {
        return;
    };
    if let Some(last) = spawn_dialog(&mut commands, root, asked) {
        standing.focus(last);
    }
}

/// The dialog under `root`, answering with its last button.
fn spawn_dialog(commands: &mut Commands, root: Entity, asked: &Asked) -> Option<Entity> {
    let rows = wrapped_rows(&asked.message);
    commands
        .entity(root)
        .insert((
            overlay::centred(WIDTH, rows + ANSWER_ROWS),
            Framed::over(TITLE),
            TabGroup::modal(),
            ModalOpen,
            Hints(&[("⇥", "switch"), ("⏎", "choose"), ("esc", "cancel")]),
        ))
        .observe(handle_key)
        .observe(handle_dismiss);
    let question = Paragraph::new(asked.message.clone()).wrap(Wrap { trim: true });
    commands.spawn((
        fixed(f32::from(rows)),
        UiWidget::new(question),
        placed(),
        ChildOf(root),
    ));
    commands.spawn((growing(), ChildOf(root)));
    let answers = spawn_button_row(commands, root);
    let mut last = None;
    for (at, answer) in asked.answers.iter().enumerate() {
        let spawned = commands
            .spawn((
                button(answer.label),
                Answers(at),
                button_node(answer.label),
                placed(),
                ChildOf(answers),
            ))
            .observe(handle_activate)
            .id();
        if let Some(emphasis) = answer.emphasis {
            commands.entity(spawned).insert(emphasis);
        }
        last = Some(spawned);
    }
    last
}

/// The rows `message` takes wrapped a word at a time, as the paragraph
/// wraps it.
fn wrapped_rows(message: &str) -> u16 {
    u16::try_from(wrapped(message, TEXT_COLS, "").len()).unwrap_or(u16::MAX)
}

fn handle_activate(
    activated: On<Activate>,
    answers: Query<&Answers>,
    mut confirm: ResMut<Confirm>,
    mut commands: Commands,
) {
    let Ok(&answer) = answers.get(activated.entity) else {
        return;
    };
    let chosen = confirm
        .0
        .take()
        .and_then(|asked| asked.answers.into_iter().nth(answer.0));
    if let Some(then) = chosen.and_then(|answer| answer.then) {
        then(&mut commands);
    }
}

fn handle_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    mut confirm: ResMut<Confirm>,
) {
    if first_bound(overlay::CLOSE_KEYS, &input.input, held.get()).is_some() {
        input.propagate(false);
        confirm.0 = None;
    }
}

/// A press outside the dialog leaves the question unanswered.
fn handle_dismiss(_dismissed: On<ModalDismiss>, mut confirm: ResMut<Confirm>) {
    confirm.0 = None;
}

#[cfg(test)]
mod tests {
    use super::{TEXT_COLS, wrapped_rows};
    use bevy_input_focus::InputFocus;
    use plurimus::term::KeyCode;

    use crate::commands::tui::edit::Draft;
    use crate::commands::tui::edit::tests::open;
    use crate::commands::tui::nav::Page;
    use crate::commands::tui::support::{
        Headless, SIZE, click, composed_frame, headless_app, is_asking, press_ctrl, press_key,
        type_text,
    };

    fn dirty_app() -> Headless {
        let mut app = headless_app(SIZE);
        open(&mut app, Page::Accounts);
        press_key(&mut app, KeyCode::Tab);
        type_text(&mut app, "x");
        press_key(&mut app, KeyCode::Enter);
        assert!(app.world().resource::<Draft>().is_dirty());
        app
    }

    #[test]
    fn an_open_item_keeps_the_chord_that_would_ask() {
        let mut app = headless_app(SIZE);
        open(&mut app, Page::Accounts);
        press_ctrl(&mut app, KeyCode::Char('c'));
        assert!(!is_asking(&app));
        assert!(app.should_exit().is_none());
    }

    #[test]
    fn an_unanswered_question_gives_the_keyboard_back_to_what_held_it() {
        let mut app = dirty_app();
        let table = app.world().resource::<InputFocus>().get();
        press_key(&mut app, KeyCode::Char('q'));
        assert!(is_asking(&app));
        assert_ne!(app.world().resource::<InputFocus>().get(), table);
        let frame = composed_frame(&app);
        assert!(frame.contains("╭ Confirm"), "{frame}");
        press_key(&mut app, KeyCode::Esc);
        assert!(!is_asking(&app));
        assert_eq!(app.world().resource::<InputFocus>().get(), table);
    }

    #[test]
    fn a_press_outside_the_dialog_leaves_the_question_unanswered() {
        let mut app = dirty_app();
        press_key(&mut app, KeyCode::Char('q'));
        assert!(is_asking(&app));
        click(&mut app, 1, 2);
        assert!(!is_asking(&app));
        assert!(app.should_exit().is_none());
    }

    #[test]
    fn a_long_question_wraps_a_word_at_a_time() {
        assert_eq!(wrapped_rows("Discard the edits?"), 1);
        let fits = "a".repeat(usize::from(TEXT_COLS));
        assert_eq!(wrapped_rows(&fits), 1);
        assert_eq!(wrapped_rows(&format!("{fits} b")), 2);
        let question = "Take the 24% ladder? 14 conversion(s), 2026–2038, in place of the ladder taken before.";
        assert_eq!(wrapped_rows(question), 3);
    }
}
