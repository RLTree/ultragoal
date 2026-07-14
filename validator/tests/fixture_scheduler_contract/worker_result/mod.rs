use crate::orchestration::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

include!("historical_result.rs");

include!("current_inputs.rs");
