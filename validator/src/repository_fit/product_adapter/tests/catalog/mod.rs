use super::scenario::{Fixture, assert_zero_write};
use crate::context::{BuildRequest, LiveContext};
use crate::repository_fit::product_adapter::AdapterErrorId;
use crate::repository_fit::product_adapter::catalog::{
    CANONICAL_TEMPLATES, TemplateCatalogRow, TemplateSourceKind, compile, compile_rows,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "../../../../../build_support/repository_fit_template_sources.rs"]
mod production_sources;

#[path = "next_source_fixture.rs"]
mod next_source_fixture;
#[path = "production_generate_erases_preexisting_outputs_before_manifest_validation.rs"]
mod production_generate_erases_preexisting_outputs_before_manifest_validation;
#[path = "production_generate_reports_cleanup_failure_before_validation.rs"]
mod production_generate_reports_cleanup_failure_before_validation;
#[path = "production_source_classifier_rejects_real_links_fifo_socket_and_directory_without_hanging.rs"]
mod production_source_classifier_rejects_real_links_fifo_socket_and_directory_without_hanging;
#[path = "traversal_absolute_and_source_target_mismatch_are_rejected.rs"]
mod traversal_absolute_and_source_target_mismatch_are_rejected;

pub(crate) use next_source_fixture::*;
pub(crate) use production_generate_erases_preexisting_outputs_before_manifest_validation::*;
pub(crate) use production_generate_reports_cleanup_failure_before_validation::*;
pub(crate) use production_source_classifier_rejects_real_links_fifo_socket_and_directory_without_hanging::*;
pub(crate) use traversal_absolute_and_source_target_mismatch_are_rejected::*;
