use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::host_capability::{
    HostCapabilityDeclaration, HostCapabilityState, JourneyBinding,
};
use crate::distribution::json;
use crate::distribution::model::{Capability, DistributionReport, Layer, LayerVerdict};
use crate::distribution::reader::sha256;
use crate::plugin_manifest::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

include!("registry/model.rs");

include!("validate_app_registry.rs");

include!("discovery_publication.rs");

include!("registry/publication.rs");

include!("observe_discovery.rs");

#[cfg(test)]
#[path = "registry/reader_race_tests.rs"]
mod reader_race_tests;
