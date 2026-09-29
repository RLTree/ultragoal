use crate::distribution::{DistributionErrorId as ErrorId, plan_package};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID, Fixture, PLUGIN_ID, VERSION, tree};
use serde_json::{Value, json};
use std::fs;

include!("manifest.rs");

include!("metadata_and_component_paths_are_bounded_normalized_and_unique.rs");
