pub mod context {
    pub use ultragoal::context::*;
}

#[path = "../src/cli/capture/mod.rs"]
pub mod capture;

#[path = "../src/routine_work/mod.rs"]
pub mod routine_work;

#[path = "../src/routine_work/catalog/mod.rs"]
mod catalog;

use catalog::{
    AdoptedRoutineNode, CatalogAdoption, CatalogSelectionRequest, ProductionRoutineCatalog,
    RunnerObservation, SelectedRoutineNode, TransitiveInputExpectation, load_production_catalog,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::fs;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "routine_production_catalog_cases/catalog_definition_rejection.rs"]
mod catalog_definition_rejection;
#[path = "routine_production_catalog_cases/catalog_fixture.rs"]
mod catalog_fixture;
#[path = "routine_production_catalog_cases/catalog_fixture_claim.rs"]
mod catalog_fixture_claim;
#[path = "routine_production_catalog_cases/catalog_fixture_cleanup_hook.rs"]
mod catalog_fixture_cleanup_hook;
#[path = "routine_production_catalog_cases/catalog_fixture_construction.rs"]
mod catalog_fixture_construction;
#[path = "routine_production_catalog_cases/catalog_fixture_custody.rs"]
mod catalog_fixture_custody;
#[path = "routine_production_catalog_cases/catalog_fixture_custody_types.rs"]
mod catalog_fixture_custody_types;
#[path = "routine_production_catalog_cases/catalog_fixture_directory_entries.rs"]
mod catalog_fixture_directory_entries;
#[path = "routine_production_catalog_cases/catalog_fixture_invocation.rs"]
mod catalog_fixture_invocation;
#[path = "routine_production_catalog_cases/catalog_fixture_invocation_controls.rs"]
mod catalog_fixture_invocation_controls;
#[path = "routine_production_catalog_cases/catalog_fixture_lifecycle.rs"]
mod catalog_fixture_lifecycle;
#[path = "routine_production_catalog_cases/catalog_fixture_quarantine.rs"]
mod catalog_fixture_quarantine;
#[path = "routine_production_catalog_cases/catalog_fixture_rebind.rs"]
mod catalog_fixture_rebind;
#[path = "routine_production_catalog_cases/catalog_fixture_scope.rs"]
mod catalog_fixture_scope;
#[path = "routine_production_catalog_cases/repository_fixture.rs"]
mod repository_fixture;
#[path = "routine_production_catalog_cases/source_path_rejection.rs"]
mod source_path_rejection;
#[path = "routine_production_catalog_cases/transitive_input_rejection.rs"]
mod transitive_input_rejection;
#[path = "routine_production_catalog_cases/unsafe_invocation_rejection.rs"]
mod unsafe_invocation_rejection;

pub(crate) use catalog_definition_rejection::*;
pub(crate) use catalog_fixture::*;
pub(crate) use repository_fixture::*;
pub(crate) use source_path_rejection::*;
pub(crate) use transitive_input_rejection::*;
pub(crate) use unsafe_invocation_rejection::*;
