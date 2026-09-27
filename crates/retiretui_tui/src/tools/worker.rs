//! Work beside the frames: on a thread of its own where there are
//! threads, and on the page between frames where there is one; how far it
//! has got, and the way to stop it, which dropping it takes.

use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::thread::JoinHandle;
use std::time::Duration;

use bevy_ecs::prelude::Resource;
use retiretui_engine::market::Progress;
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
type Worker<T> = Threaded<T>;
#[cfg(target_arch = "wasm32")]
type Worker<T> = Deferred<T>;

#[cfg(not(target_arch = "wasm32"))]
struct Threaded<T> {
    /// Taken once the thread has answered.
    handle: Option<JoinHandle<T>>,
    progress: Arc<Progress>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + 'static> Threaded<T> {
    fn spawn(work: impl FnOnce(&Progress) -> T + Send + 'static) -> Self {
        let progress = Arc::new(Progress::default());
        let shared = Arc::clone(&progress);
        let handle = std::thread::spawn(move || work(&shared));
        Self {
            handle: Some(handle),
            progress,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T> Threaded<T> {
    fn is_finished(&self) -> bool {
        self.handle.as_ref().is_some_and(JoinHandle::is_finished)
    }

    /// What the thread answered; none where it panicked.
    fn join(mut self) -> Option<T> {
        self.handle.take()?.join().ok()
    }
}

/// Work no longer wanted stops with the steps under way.
#[cfg(not(target_arch = "wasm32"))]
impl<T> Drop for Threaded<T> {
    fn drop(&mut self) {
        self.progress.cancel();
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
type Work<T> = Box<dyn FnOnce(&Progress) -> T + Send>;

/// Work kept to run on the thread that polls it: the first poll only
/// sees it, so the frame that shows it under way is drawn, and the next
/// runs it through. Dropped before then, it never runs.
#[cfg(any(test, target_arch = "wasm32"))]
struct Deferred<T> {
    stage: std::sync::Mutex<Stage<T>>,
    progress: Arc<Progress>,
}

#[cfg(any(test, target_arch = "wasm32"))]
enum Stage<T> {
    Kept(Work<T>),
    Seen(Work<T>),
    Answered(T),
    Taken,
}

#[cfg(any(test, target_arch = "wasm32"))]
impl<T: Send + 'static> Deferred<T> {
    fn spawn(work: impl FnOnce(&Progress) -> T + Send + 'static) -> Self {
        Self {
            stage: std::sync::Mutex::new(Stage::Kept(Box::new(work))),
            progress: Arc::new(Progress::default()),
        }
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
impl<T> Deferred<T> {
    fn is_finished(&self) -> bool {
        let Ok(mut stage) = self.stage.lock() else {
            return false;
        };
        *stage = match std::mem::replace(&mut *stage, Stage::Taken) {
            Stage::Kept(work) => Stage::Seen(work),
            Stage::Seen(work) => Stage::Answered(work(&self.progress)),
            answered => answered,
        };
        matches!(*stage, Stage::Answered(_))
    }

    /// What the work answered, once a poll has run it.
    fn join(self) -> Option<T> {
        match self.stage.into_inner().ok()? {
            Stage::Answered(answer) => Some(answer),
            Stage::Kept(_) | Stage::Seen(_) | Stage::Taken => None,
        }
    }
}

/// Work under way over `key`, which says what it is for, and since when.
pub(crate) struct Keyed<K, T> {
    pub(crate) key: K,
    worker: Worker<T>,
    since: Instant,
}

impl<K, T: Send + 'static> Keyed<K, T> {
    pub(crate) fn spawn(key: K, work: impl FnOnce(&Progress) -> T + Send + 'static) -> Self {
        Self {
            key,
            worker: Worker::spawn(work),
            since: Instant::now(),
        }
    }
}

impl<K, T> Keyed<K, T> {
    pub(crate) fn is_finished(&self) -> bool {
        self.worker.is_finished()
    }

    pub(crate) fn progress(&self) -> &Progress {
        &self.worker.progress
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.since.elapsed()
    }

    /// The key, what the work answered - none where it panicked - and
    /// how long it took.
    pub(crate) fn join(self) -> (K, Option<T>, Duration) {
        let took = self.since.elapsed();
        (self.key, self.worker.join(), took)
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    #[test]
    fn deferred_work_runs_on_the_poll_after_the_one_that_sees_it() {
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);
        let work = Deferred::spawn(move |_| flag.store(true, Ordering::Relaxed));
        assert!(!work.is_finished(), "the first poll only sees it");
        assert!(!ran.load(Ordering::Relaxed));
        assert!(work.is_finished());
        assert!(ran.load(Ordering::Relaxed));
        assert_eq!(work.join(), Some(()));
    }

    #[test]
    fn deferred_work_dropped_first_never_runs() {
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);
        let work = Deferred::spawn(move |_| flag.store(true, Ordering::Relaxed));
        assert!(!work.is_finished());
        drop(work);
        assert!(!ran.load(Ordering::Relaxed));
    }
}
