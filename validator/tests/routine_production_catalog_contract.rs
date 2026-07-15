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
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "routine_production_catalog_cases/catalog_definition_rejection.rs"]
mod catalog_definition_rejection;
#[path = "routine_production_catalog_cases/catalog_fixture/mod.rs"]
mod catalog_fixture;
#[path = "routine_production_catalog_cases/catalog_fixture/claim.rs"]
mod claim;
#[path = "routine_production_catalog_cases/catalog_fixture/cleanup_hook.rs"]
mod cleanup_hook;
#[path = "routine_production_catalog_cases/catalog_fixture/construction.rs"]
mod construction;
#[path = "routine_production_catalog_cases/catalog_fixture/custody.rs"]
mod custody;
#[path = "routine_production_catalog_cases/catalog_fixture/custody_types.rs"]
mod custody_types;
#[path = "routine_production_catalog_cases/catalog_fixture/directory_entries.rs"]
mod directory_entries;
#[path = "routine_production_catalog_cases/catalog_fixture/invocation.rs"]
mod invocation;
#[path = "routine_production_catalog_cases/catalog_fixture/invocation_controls.rs"]
mod invocation_controls;
#[path = "routine_production_catalog_cases/catalog_fixture/lifecycle.rs"]
mod lifecycle;
#[path = "routine_production_catalog_cases/catalog_fixture/quarantine.rs"]
mod quarantine;
#[path = "routine_production_catalog_cases/catalog_fixture/rebind.rs"]
mod rebind;
#[path = "routine_production_catalog_cases/repository_fixture.rs"]
mod repository_fixture;
#[path = "routine_production_catalog_cases/catalog_fixture/scope.rs"]
mod scope;
#[path = "routine_production_catalog_cases/source_path_rejection.rs"]
mod source_path_rejection;
#[path = "routine_production_catalog_cases/transitive_input_rejection.rs"]
mod transitive_input_rejection;
#[path = "routine_production_catalog_cases/unsafe_invocation_rejection.rs"]
mod unsafe_invocation_rejection;

pub(crate) use catalog_fixture::*;
pub(crate) use repository_fixture::*;
