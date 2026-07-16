mod content_digest_adapter;
mod descriptor;
mod descriptor_parser;

#[cfg(unix)]
mod anchored;
#[cfg(not(unix))]
mod unsupported;

use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use std::path::{Component, Path};

#[cfg(unix)]
pub(crate) use anchored::{AnchoredDirectory, AnchoredRoot, SecureFile};
#[cfg(all(test, unix))]
pub(crate) use anchored::{
    ReaddirTestFault, TestIoCounts, reset_test_io_counts, set_test_readdir_fault, test_io_counts,
    test_readdir_fault_triggered,
};
pub(crate) use descriptor::parse_descriptor;
#[cfg(not(unix))]
pub(crate) use unsupported::{AnchoredDirectory, AnchoredRoot, SecureFile};

pub(crate) fn digest(bytes: &[u8]) -> String {
    content_digest_adapter::sha256(bytes)
}

pub(crate) fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}

pub(crate) fn safe_relative(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(value) if !value.is_empty()))
}

fn unsafe_entry() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::UnsafeFilesystemEntry)
}

fn invalid_source() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidSourceCatalog)
}

fn changed() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationChanged)
}

fn too_large() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InputTooLarge)
}
