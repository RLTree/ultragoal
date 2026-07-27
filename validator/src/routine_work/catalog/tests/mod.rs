use super::{
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

#[path = "cases/catalog_binding.rs"]
mod catalog_binding;
#[path = "cases/catalog_definition_rejection.rs"]
mod catalog_definition_rejection;
#[path = "cases/catalog_fixture/mod.rs"]
mod catalog_fixture;
mod catalog_scope_construction;
#[path = "cases/repository_fixture.rs"]
mod repository_fixture;
#[path = "cases/source_path_rejection.rs"]
mod source_path_rejection;
#[path = "cases/transitive_input_rejection.rs"]
mod transitive_input_rejection;
#[path = "cases/unsafe_invocation_rejection.rs"]
mod unsafe_invocation_rejection;

pub(crate) use catalog_binding::*;
pub(crate) use catalog_fixture::VALID_CATALOG;
pub(crate) use repository_fixture::*;
