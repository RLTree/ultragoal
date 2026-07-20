use super::*;
use crate::context::{BuildRequest, LiveContext};
use crate::distribution::package::{ExpectedTree, TreeObject, tree_sha256};
use crate::distribution::{
    ConfinedRoot, HostCapabilityDeclaration, IdentitySurface, JourneyBinding, ScopedFile,
    ScopedTree, SurfaceIdentity,
};
use crate::distribution::{
    EffectPoint, assert_test_effect_hook_consumed, set_test_effect_hook_matching,
};
use crate::inventory::{AuthorityCatalog, AuthorityCatalogDefinition, GeneratedSurfaceIndex};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier};
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
#[cfg(unix)]
use std::os::unix::net::UnixListener;

include!("repo.rs");

include!("repository_snapshot.rs");

include!("deterministic_capture.rs");

include!("undeclared_secret_local_and_benign_skill_members_fail_closed.rs");

include!("source_mutate_restore_during_publication_rolls_back_output.rs");

include!("inventory_publication.rs");

include!("every_artifact_binding_dimension_is_checked_before_output_effects.rs");

include!("package_surface_identity.rs");

include!("artifact_identity_fields.rs");
