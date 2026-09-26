//! What every search the engine runs shares: how far it has got, a way to
//! stop it, why it answered nothing, and the machine's threads to run its
//! steps on.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use crate::plan::Issue;

/// How far a search has got, and a way to stop it. Shared with the thread
/// that runs it.
#[derive(Debug, Default)]
pub struct Progress {
    done: AtomicUsize,
    is_cancelled: AtomicBool,
}

impl Progress {
    /// How many steps of the search have finished.
    #[must_use]
    pub fn done(&self) -> usize {
        self.done.load(Ordering::Relaxed)
    }

    /// Asks the search to stop after the steps under way.
    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::Relaxed);
    }

    /// Whether the search has been asked to stop, which work of its own
    /// between steps may check.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::Relaxed)
    }
}

/// Why a search answered nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum RunError {
    /// It was cancelled.
    Cancelled,
    /// The plan's settings cannot be run, and why.
    Refused(Vec<Issue>),
}

/// The steps at `range`, stopping early once `progress` is cancelled.
fn run_range<T>(
    range: std::ops::Range<usize>,
    progress: &Progress,
    one: &(impl Fn(usize) -> Option<T> + Sync),
) -> Vec<Option<T>> {
    range
        .take_while(|_| !progress.is_cancelled())
        .map(|at| {
            let step = one(at);
            progress.done.fetch_add(1, Ordering::Relaxed);
            step
        })
        .collect()
}

/// Runs `count` steps of a search across the machine's threads, answering
/// them in index order; each index's answer is its own whatever thread
/// takes it, and a step that answers nothing is left out.
pub(crate) fn run_all<T: Send>(
    count: usize,
    progress: &Progress,
    one: impl Fn(usize) -> Option<T> + Sync,
) -> Result<Vec<T>, RunError> {
    let threads = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(count.max(1));
    let chunk = count.div_ceil(threads).max(1);
    let one = &one;
    let finished: Vec<Vec<Option<T>>> = thread::scope(|scope| {
        let handles: Vec<_> = (0..count)
            .step_by(chunk)
            .map(|first| {
                let range = first..(first + chunk).min(count);
                scope.spawn(move || run_range(range, progress, one))
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a search step does not panic"))
            .collect()
    });
    if progress.is_cancelled() {
        return Err(RunError::Cancelled);
    }
    Ok(finished.into_iter().flatten().flatten().collect())
}
