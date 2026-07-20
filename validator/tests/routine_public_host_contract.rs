#![cfg(target_vendor = "apple")]
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

mod routine_work {
    use super::PathBuf;

    pub(crate) struct RoutineCustodyCapability;

    impl RoutineCustodyCapability {
        pub(crate) fn issue_from_host(_: PathBuf) -> Self {
            Self
        }
    }
}

mod state {
    use super::{Deserialize, Serialize};

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    pub(crate) struct RoutineFindingBinding {
        pub finding_id: String,
        pub repair_id: String,
    }
}

#[path = "../src/cli/successor_public/routine/host/host_failure.rs"]
mod host_failure;
#[path = "../src/cli/successor_public/routine/host/supported/mod.rs"]
mod supported;

pub(crate) use host_failure::*;
