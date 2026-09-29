#![cfg(target_vendor = "apple")]
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) struct HostCustodyIssuance(PathBuf);

impl HostCustodyIssuance {
    fn new(authority_root: PathBuf) -> Self {
        Self(authority_root)
    }
}

mod routine_work {
    use super::{Deserialize, Serialize};

    pub(crate) struct RoutineCustodyCapability;

    impl RoutineCustodyCapability {
        pub(crate) fn issue_from_host(_: super::HostCustodyIssuance) -> Self {
            Self
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    pub(crate) enum RoutineTerminalOutcome {
        Complete,
        Failed,
        Cancelled,
        Incomplete,
        Ambiguous,
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
