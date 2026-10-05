//! What every search the engine runs shares: how far it has got, a way to
//! stop it, why it answered nothing, and the machine's threads to run its
//! steps on.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(not(target_arch = "wasm32"))]
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

impl RunError {
    /// A refusal for the one reason `message`, at `path`.
    pub(crate) fn refused(path: &str, message: &str) -> Self {
        Self::Refused(vec![Issue {
            path: path.to_owned(),
            message: message.to_owned(),
        }])
    }
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
    let finished = run_chunks(count, progress, &one);
    if progress.is_cancelled() {
        return Err(RunError::Cancelled);
    }
    Ok(finished.into_iter().flatten().flatten().collect())
}

#[cfg(not(target_arch = "wasm32"))]
fn run_chunks<T: Send>(
    count: usize,
    progress: &Progress,
    one: &(impl Fn(usize) -> Option<T> + Sync),
) -> Vec<Vec<Option<T>>> {
    let threads = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(count.max(1));
    let chunk = count.div_ceil(threads).max(1);
    thread::scope(|scope| {
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
    })
}

/// A browser page has one thread, and spawning another panics.
#[cfg(target_arch = "wasm32")]
fn run_chunks<T: Send>(
    count: usize,
    progress: &Progress,
    one: &(impl Fn(usize) -> Option<T> + Sync),
) -> Vec<Vec<Option<T>>> {
    vec![run_range(0..count, progress, one)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_thread_answers_what_many_do() {
        let square = |at: usize| (!at.is_multiple_of(3)).then_some(at * at);
        let progress = Progress::default();
        let serial: Vec<usize> = run_range(0..40, &progress, &square)
            .into_iter()
            .flatten()
            .collect();
        let threaded = run_all(40, &Progress::default(), square).unwrap();
        assert_eq!(serial, threaded);
        assert_eq!(progress.done(), 40);
    }
}
