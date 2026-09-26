//! The picker: a query over a list of rows, which knows nothing of what
//! its rows are. A caller registers the system that lists them and the
//! system that is told which was chosen.

mod matching;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Changed, Commands, Component, Entity, In, IntoScheduleConfigs, IntoSystem, Local, On, Query,
    Res, ResMut, Resource, With, World,
};
use bevy_ecs::system::SystemId;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input_focus::FocusedInput;
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::layout::Size;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{ModalDismiss, ModalOpen, ScrollArea, UiStyle, first_bound};
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;
use plurimus::widgets::{
    ActiveDescendant, ListBoxAction, ListBoxKeys, ListItemTrailing, TextInput, TextInputKeys,
    ValueChange, list_item, listbox,
};

pub use matching::ranked;

use super::hints::Hints;
use super::layout::{fixed, growing, list_cursor, placed};
use super::overlay::{self, Standing};
use super::pane::Framed;
use super::theme::Theme;

pub fn plugin(app: &mut App) {
    app.init_resource::<Picking>();
    app.add_systems(
        Update,
        (sync_picker, relist, try_on)
            .chain()
            .in_set(overlay::Settles),
    );
}

const WIDTH: u16 = 64;
const LIST_ROWS: u16 = 10;
/// The query row, and the rows of the list under it.
const ROWS: u16 = 1 + LIST_ROWS;
pub const PROMPT: &str = "> ";
const CARET: &str = "█";
/// One row a picker offers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Offered {
    /// What the caller knows the row by.
    pub id: usize,
    /// What the row says, and what a query is matched against.
    pub text: String,
    /// Said dimly at the row's trailing end.
    pub badge: String,
    /// Whether the whole row is said dimly, as one not worth choosing.
    pub is_dim: bool,
    indices: Vec<u32>,
}

impl Offered {
    #[must_use]
    pub fn new(id: usize, text: impl Into<String>) -> Self {
        Self {
            id,
            text: text.into(),
            badge: String::new(),
            is_dim: false,
            indices: Vec::new(),
        }
    }

    #[must_use]
    pub fn badged(mut self, badge: impl Into<String>) -> Self {
        self.badge = badge.into();
        self
    }

    #[must_use]
    pub fn dimmed(mut self) -> Self {
        self.is_dim = true;
        self
    }
}

/// One kind of picker: what it is called, and the systems it asks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Picker {
    title: &'static str,
    /// What one row is, for saying that none matches.
    noun: &'static str,
    list: SystemId<In<String>, Vec<Offered>>,
    chosen: SystemId<In<usize>>,
    trying: Option<Trying>,
}

/// How a picker lets a row be tried before it is chosen: `reached` is told
/// the row the cursor comes to rest on, and `closed` puts things back when
/// the picker closes with nothing chosen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Trying {
    reached: SystemId<In<usize>>,
    closed: SystemId,
}

impl Picker {
    /// Registers `list`, which answers a query with the rows it leaves,
    /// and `chosen`, which is told the id of the row picked.
    pub fn new<ListMarker, ChosenMarker>(
        world: &mut World,
        (title, noun): (&'static str, &'static str),
        list: impl IntoSystem<In<String>, Vec<Offered>, ListMarker> + 'static,
        chosen: impl IntoSystem<In<usize>, (), ChosenMarker> + 'static,
    ) -> Self {
        Self {
            title,
            noun,
            list: world.register_system(list),
            chosen: world.register_system(chosen),
            trying: None,
        }
    }

    /// The same picker, trying each row on as the cursor reaches it.
    #[must_use]
    pub fn trying<ReachedMarker, ClosedMarker>(
        mut self,
        world: &mut World,
        reached: impl IntoSystem<In<usize>, (), ReachedMarker> + 'static,
        closed: impl IntoSystem<(), (), ClosedMarker> + 'static,
    ) -> Self {
        self.trying = Some(Trying {
            reached: world.register_system(reached),
            closed: world.register_system(closed),
        });
        self
    }
}

/// The picker on show, and what has been typed into it.
#[derive(Resource, Default, Debug)]
pub struct Picking {
    open: Option<Picker>,
    query: String,
    is_stale: bool,
}

impl Picking {
    pub fn open(&mut self, picker: Picker) {
        self.open = Some(picker);
        self.query.clear();
        self.is_stale = true;
    }

    pub fn close(&mut self) {
        self.open = None;
    }

    /// Closes with nothing chosen, putting back what was tried on.
    fn abandon(&mut self, commands: &mut Commands) {
        if let Some(trying) = self.open.and_then(|picker| picker.trying) {
            commands.run_system(trying.closed);
        }
        self.close();
    }

    #[cfg(test)]
    pub const fn is_open(&self) -> bool {
        self.open.is_some()
    }
}

#[derive(Component, Default, Debug)]
struct PickerRoot;

/// The query row: the text typed, held by a field that never takes the
/// keyboard.
#[derive(Component)]
struct Field;

#[derive(Component)]
struct Results;

/// The id of the offered row a list item stands for.
#[derive(Component, Clone, Copy)]
struct Row(usize);

fn sync_picker(
    picking: Res<Picking>,
    mut shown: Local<Option<Picker>>,
    mut standing: Standing<PickerRoot>,
    mut commands: Commands,
) {
    if !picking.is_changed() || *shown == picking.open {
        return;
    }
    *shown = picking.open;
    let Some(picker) = picking.open else {
        standing.close(&mut commands);
        return;
    };
    let Some(root) = standing.open(&mut commands) else {
        return;
    };
    commands
        .entity(root)
        .insert((
            overlay::centred(WIDTH, ROWS),
            Framed::over(picker.title),
            ModalOpen,
            Hints(&[("↑↓", "move"), ("⏎", "pick"), ("esc", "close")]),
        ))
        .observe(handle_key)
        .observe(handle_dismiss);
    commands.spawn((
        Field,
        TextInput::new(""),
        TextInputKeys::default(),
        fixed(1.0),
        UiWidget::default(),
        placed(),
        ChildOf(root),
    ));
    let results = commands
        .spawn((
            Results,
            listbox(),
            list_cursor(),
            list_keys(),
            ScrollArea::new(Size::default()),
            growing(),
            placed(),
            ChildOf(root),
        ))
        .observe(handle_chosen)
        .id();
    standing.focus(results);
}

/// The list's keys without space, which belongs to the query.
fn list_keys() -> ListBoxKeys {
    ListBoxKeys(vec![
        (Key::ArrowUp.into(), ListBoxAction::Up),
        (Key::ArrowDown.into(), ListBoxAction::Down),
        (Key::PageUp.into(), ListBoxAction::PageUp),
        (Key::PageDown.into(), ListBoxAction::PageDown),
        (Key::Enter.into(), ListBoxAction::Select),
    ])
}

/// Asks the picker's list system what the query leaves and redraws the
/// rows and the query row. Exclusive because the list is a one-shot.
fn relist(world: &mut World) {
    let picking = world.resource::<Picking>();
    let (true, Some(picker)) = (picking.is_stale, picking.open) else {
        return;
    };
    let query = picking.query.clone();
    let Ok(results) = world
        .query_filtered::<Entity, With<Results>>()
        .single(world)
    else {
        return;
    };
    world.resource_mut::<Picking>().is_stale = false;
    let rows = world
        .run_system_with(picker.list, query.clone())
        .unwrap_or_default();
    let theme = world.resource::<Theme>().clone();
    world.entity_mut(results).despawn_related::<Children>();
    let first = spawn_rows(world, results, &rows, &theme);
    if rows.is_empty() {
        say_no_match(world, results, picker, &query, &theme);
    }
    world.entity_mut(results).insert(ActiveDescendant(first));
    draw_query(world, query, &theme);
}

fn draw_query(world: &mut World, query: String, theme: &Theme) {
    let typed = Line::from(vec![
        Span::styled(PROMPT, theme.dimmed()),
        Span::raw(query),
        Span::styled(CARET, theme.accented()),
    ]);
    if let Ok(mut field) = world
        .query_filtered::<&mut UiWidget, With<Field>>()
        .single_mut(world)
    {
        *field = UiWidget::new(Paragraph::new(typed));
    }
}

/// The rows under `results`, and the first, which the cursor opens on.
fn spawn_rows(
    world: &mut World,
    results: Entity,
    rows: &[Offered],
    theme: &Theme,
) -> Option<Entity> {
    let mut first = None;
    for row in rows {
        let label = matching::lit_line(&row.text, &row.indices, theme.accented());
        let trailing = Line::styled(row.badge.clone(), theme.dimmed());
        let mut item = world.spawn((
            list_item(label),
            ListItemTrailing(trailing),
            Row(row.id),
            ChildOf(results),
        ));
        if row.is_dim {
            item.insert(UiStyle(theme.dimmed()));
        }
        let item = item.id();
        first.get_or_insert(item);
    }
    first
}

fn say_no_match(world: &mut World, results: Entity, picker: Picker, query: &str, theme: &Theme) {
    let said = format!("no {} matches {query:?}", picker.noun);
    world.spawn((list_item(said), UiStyle(theme.dimmed()), ChildOf(results)));
}

/// Tells a picker that tries rows on which one the cursor has reached.
fn try_on(
    picking: Res<Picking>,
    lists: Query<&ActiveDescendant, (With<Results>, Changed<ActiveDescendant>)>,
    rows: Query<&Row>,
    mut commands: Commands,
) {
    let Some(trying) = picking.open.and_then(|picker| picker.trying) else {
        return;
    };
    let reached = lists.single().ok().and_then(|cursor| cursor.0);
    if let Some(row) = reached.and_then(|row| rows.get(row).ok()) {
        commands.run_system_with(trying.reached, row.0);
    }
}

/// Esc closes; a key the list did not take is typed into the query.
fn handle_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    mut fields: Query<(&mut TextInput, &TextInputKeys), With<Field>>,
    mut picking: ResMut<Picking>,
    mut commands: Commands,
) {
    if first_bound(overlay::CLOSE_KEYS, &input.input, held.get()).is_some() {
        input.propagate(false);
        picking.abandon(&mut commands);
        return;
    }
    let Ok((mut field, keys)) = fields.single_mut() else {
        return;
    };
    if !field.handle(keys, &input.input, held.get()) {
        return;
    }
    input.propagate(false);
    if field.value() != picking.query {
        field.value().clone_into(&mut picking.query);
        picking.is_stale = true;
    }
}

fn handle_chosen(
    chosen: On<ValueChange<Entity>>,
    rows: Query<&Row>,
    mut picking: ResMut<Picking>,
    mut commands: Commands,
) {
    let Some(picker) = picking.open else {
        return;
    };
    if let Ok(row) = rows.get(chosen.value) {
        picking.close();
        commands.run_system_with(picker.chosen, row.0);
    }
}

fn handle_dismiss(
    _dismissed: On<ModalDismiss>,
    mut picking: ResMut<Picking>,
    mut commands: Commands,
) {
    picking.abandon(&mut commands);
}
