//! The retirement planner in a browser page, over a workspace kept in the
//! page's own storage, which never leaves the visitor's browser. Built for
//! `wasm32-unknown-unknown` alone; elsewhere it is empty.

#[cfg(target_arch = "wasm32")]
mod page;
#[cfg(target_arch = "wasm32")]
mod storage;
