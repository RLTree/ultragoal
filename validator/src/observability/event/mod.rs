use super::limits::{EVENT_SCHEMA, MAX_ATTRIBUTES, MAX_DURATION_MS, MAX_REFERENCES, MAX_ROW_BYTES};
use super::privacy;
use crate::capture::CapturedRun;
use crate::context::LiveContext;
use crate::state::Finding;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[path = "event_attributes.rs"]
mod event_attributes;
#[path = "event_construction.rs"]
mod event_construction;
#[path = "event_record.rs"]
mod event_record;

pub use event_construction::SemanticEventInput;
pub use event_record::*;
