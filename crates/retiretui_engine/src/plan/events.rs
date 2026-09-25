use serde::{Deserialize, Serialize};

use super::triggers::Trigger;

/// A named milestone other plan items hang triggers on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// Unique id triggers reference as `event`.
    pub id: String,
    /// Display name; defaults to the id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// When the event fires.
    pub trigger: Trigger,
}
