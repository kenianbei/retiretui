//! What the program has said to its user. Every message is a `tracing`
//! event under one target, so there is one record with two readers: the
//! journal kept here, and the log file.

use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use bevy_app::{App, Update};
use bevy_ecs::prelude::{
    IntoScheduleConfigs, Message, MessageReader, MessageWriter, Res, ResMut, Resource,
};
use bevy_ecs::schedule::SystemSet;
use jiff::Zoned;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// The target a message meant for the user is said under.
pub const USER_TARGET: &str = "retiretui::user";

const MESSAGE_FIELD: &str = "message";
const CAPACITY: usize = 1000;

/// Says `text` to the user as something that went as asked.
pub fn say(text: impl AsRef<str>) {
    tracing::info!(target: USER_TARGET, "{}", text.as_ref());
}

/// Says `text` to the user as something refused, or gone wrong.
pub fn warn(text: impl AsRef<str>) {
    tracing::warn!(target: USER_TARGET, "{}", text.as_ref());
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub at: Zoned,
    pub level: Level,
    pub text: String,
}

/// An entry as it reaches the app, for whoever shows it as it is said.
#[derive(Message, Clone, Debug)]
pub struct Said(pub Entry);

/// Where the `tracing` layer leaves what it heard, on whatever thread it
/// heard it, for the app to take up on its own.
#[derive(Resource, Clone, Debug, Default)]
pub struct Inbox(Arc<Mutex<Vec<Entry>>>);

impl Inbox {
    fn record(&self, entry: Entry) {
        if let Ok(mut pending) = self.0.lock() {
            pending.push(entry);
        }
    }

    fn take(&self) -> Vec<Entry> {
        self.0
            .lock()
            .map(|mut pending| std::mem::take(&mut *pending))
            .unwrap_or_default()
    }
}

/// Everything said this session, oldest first, the oldest dropped past a
/// bound.
#[derive(Resource, Debug, Default)]
pub struct Journal(VecDeque<Entry>);

impl Journal {
    pub fn entries(&self) -> impl ExactSizeIterator<Item = &Entry> {
        self.0.iter()
    }

    fn record(&mut self, entry: Entry) {
        if self.0.len() == CAPACITY {
            self.0.pop_front();
        }
        self.0.push_back(entry);
    }
}

/// The layer that hears what is said to the user and leaves it in `inbox`.
pub fn layer<S: Subscriber>(inbox: Inbox) -> impl Layer<S> {
    Recorder(inbox)
}

struct Recorder(Inbox);

impl<S: Subscriber> Layer<S> for Recorder {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        if event.metadata().target() != USER_TARGET {
            return;
        }
        let mut said = Words::default();
        event.record(&mut said);
        self.0.record(Entry {
            at: Zoned::now(),
            level: *event.metadata().level(),
            text: said.text,
        });
    }
}

#[derive(Default)]
struct Words {
    text: String,
}

impl Visit for Words {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == MESSAGE_FIELD {
            self.text = format!("{value:?}");
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Inbox>();
    app.init_resource::<Journal>();
    app.add_message::<Said>();
    app.add_systems(Update, (speak, record).chain().in_set(Spoken));
}

/// What was said reaching the app. Whatever shows a message as it is said
/// runs after this.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Spoken;

fn speak(inbox: Res<Inbox>, mut said: MessageWriter<Said>) {
    for entry in inbox.take() {
        said.write(Said(entry));
    }
}

fn record(mut said: MessageReader<Said>, mut journal: ResMut<Journal>) {
    for Said(entry) in said.read() {
        journal.record(entry.clone());
    }
}
