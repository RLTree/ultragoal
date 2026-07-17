use serde_json::{Value, json};
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::process::Command;

mod receipt_status;
#[cfg(test)]
mod tests;

pub(crate) use receipt_status::receipt_state;

#[cfg(test)]
#[path = "components/output_summary.rs"]
mod output_summary;
#[path = "components/state_projection.rs"]
mod state_projection;

#[cfg(test)]
pub(crate) use output_summary::*;
pub(crate) use state_projection::*;
