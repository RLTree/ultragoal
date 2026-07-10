use super::{NODE_TIMING_REL, VALIDATION_CACHE_REL};
use crate::cli::live_loop::LiveLoopCommand;
use serde_json::Value;
use std::path::{Path, PathBuf};

const LINE_CAP_CHECK_RECEIPT_REL: &str = "validation_artifacts/observability/line-cap-check.json";

pub(super) struct ReplayStore {
    pub(super) root: PathBuf,
    pub(super) values: Vec<Value>,
}

impl ReplayStore {
    pub(super) fn load(root: &Path, command: &LiveLoopCommand) -> Self {
        if command.cache_mode != "verified-local" {
            return Self {
                root: root.to_path_buf(),
                values: Vec::new(),
            };
        }
        let values = [
            VALIDATION_CACHE_REL,
            NODE_TIMING_REL,
            LINE_CAP_CHECK_RECEIPT_REL,
        ]
        .into_iter()
        .filter_map(|rel| crate::json_boundary::read_json(&root.join(rel)).ok())
        .collect();
        Self {
            root: root.to_path_buf(),
            values,
        }
    }
}
