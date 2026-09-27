//! What every interface of the retirement planner shares over the engine,
//! and draws in its own way: the words a plan is said in, the
//! load-and-validate gate, and where plan files are kept. Internal to the
//! planner's own crates; it makes no promise of a stable API.

pub mod actions;
pub mod codec;
pub mod draft;
#[cfg(feature = "native")]
pub mod environment;
pub mod files;
pub mod forms;
pub mod issues;
pub mod ladder;
pub mod metric;
pub mod present;
pub mod replies;
pub mod searches;
pub mod session;
pub mod setup;
pub mod statement;
pub mod store;
pub mod table;
