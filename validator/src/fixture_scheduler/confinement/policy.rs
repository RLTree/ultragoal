use crate::fixture_scheduler::FixtureScheduleError;
use std::time::Duration;

const MIN_MEMORY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_MEMORY_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAX_CPU_SECONDS: u64 = 600;
const MAX_WALL_MILLIS: u64 = 24 * 60 * 60 * 1000;
const MAX_FILE_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkIsolation {
    DenyAll,
}

impl NetworkIsolation {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::DenyAll => "deny-all",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfinementPolicy {
    pub cpu_seconds: u64,
    /// Additional virtual-address budget above the preparing Darwin process baseline.
    pub address_space_bytes: u64,
    pub wall_time_millis: u64,
    pub maximum_file_bytes: u64,
    pub require_process_group: bool,
    pub network: NetworkIsolation,
}

impl ConfinementPolicy {
    pub fn strict() -> Self {
        Self {
            cpu_seconds: 30,
            address_space_bytes: 512 * 1024 * 1024,
            wall_time_millis: 60_000,
            maximum_file_bytes: 64 * 1024 * 1024,
            require_process_group: true,
            network: NetworkIsolation::DenyAll,
        }
    }

    pub fn validate(&self) -> Result<(), FixtureScheduleError> {
        if !(1..=MAX_CPU_SECONDS).contains(&self.cpu_seconds)
            || !(MIN_MEMORY_BYTES..=MAX_MEMORY_BYTES).contains(&self.address_space_bytes)
            || !(1..=MAX_WALL_MILLIS).contains(&self.wall_time_millis)
            || !(1..=MAX_FILE_BYTES).contains(&self.maximum_file_bytes)
            || !self.require_process_group
        {
            return Err(FixtureScheduleError::InvalidMetadata(
                "fixture confinement policy is outside supported bounds".to_owned(),
            ));
        }
        Ok(())
    }

    pub(crate) fn wall_time(&self) -> Duration {
        Duration::from_millis(self.wall_time_millis)
    }

    pub(crate) fn digest_fragment(&self) -> String {
        format!(
            "cpu={};as={};wall={};file={};pgroup={};children=deny;network={}",
            self.cpu_seconds,
            self.address_space_bytes,
            self.wall_time_millis,
            self.maximum_file_bytes,
            self.require_process_group,
            self.network.label()
        )
    }
}
