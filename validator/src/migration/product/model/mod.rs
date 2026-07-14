use super::super::{
    InventorySurface, MigrationInventory, SurfaceFileKind, SurfaceStatus, digest, safe_reference,
    safe_relative_path, valid_identifier, valid_sha256, valid_stable_identifier,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

include!("registry_path.rs");

include!("migration_input_binding_issue.rs");

include!("compatibility/observation.rs");

include!("compatibility/contract.rs");

include!("authority_snapshot_from_surface.rs");

include!("planned/issuance.rs");

include!("compatibility/binding.rs");

include!("planned/digest.rs");

include!("product/plan.rs");

include!("product/digest.rs");
