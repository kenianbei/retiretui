//! Work on a thread of its own: how far it has got, and the way to stop
//! it, which dropping it takes.

use std::sync::Arc;
use std::thread::JoinHandle;

use bevy_ecs::prelude::Resource;
use retiretui_engine::market::Progress;

pub(crate) struct Worker<T> {
    /// Taken once the thread has answered.
    handle: Option<JoinHandle<T>>,
    pub(crate) progress: Arc<Progress>,
}

impl<T: Send + 'static> Worker<T> {
    pub(crate) fn spawn(work: impl FnOnce(&Progress) -> T + Send + 'static) -> Self {
        let progress = Arc::new(Progress::default());
        let shared = Arc::clone(&progress);
        let handle = std::thread::spawn(move || work(&shared));
        Self {
            handle: Some(handle),
            progress,
        }
    }
}

impl<T> Worker<T> {
    pub(crate) fn is_finished(&self) -> bool {
        self.handle.as_ref().is_some_and(JoinHandle::is_finished)
    }

    /// What the thread answered; none where it panicked.
    pub(crate) fn join(mut self) -> Option<T> {
        self.handle.take()?.join().ok()
    }
}

/// Work no longer wanted stops with the steps under way.
impl<T> Drop for Worker<T> {
    fn drop(&mut self) {
        self.progress.cancel();
    }
}

/// Whether the Overview runs its searches on threads of their own while
/// it is shown. The shell always does; a headless test, most of which
/// open on the Overview, turns it on only where it looks at what they
/// find.
#[derive(Resource, Clone, Copy, Debug)]
pub struct Searches(pub bool);

impl Default for Searches {
    fn default() -> Self {
        Self(true)
    }
}
