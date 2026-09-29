use crate::cli::successor_public::routine::{HostEventStore, HostState};
use crate::context::LiveContext;
use crate::observability::{CausalExplanation, EventQuery, EventStore, SemanticEvent};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(super) const RUNTIME_SOURCE_ID: &str = "successor-runtime";

mod failure;
mod terminal;
pub(super) use failure::LocalStoreFailure;
#[allow(unused_imports)]
pub(super) use terminal::{append_routine_terminal, routine_observations_from_events};

pub(super) struct LocalStore {
    home: PathBuf,
    target: PathBuf,
    context_id: String,
    candidate_id: String,
    source_id: String,
    state: Option<HostState>,
    store: Option<HostEventStore>,
}

impl LocalStore {
    pub(super) fn open(
        home: Option<&Path>,
        target: &Path,
        context: &LiveContext,
        source_id: &str,
    ) -> Result<Self, LocalStoreFailure> {
        let home = home.ok_or_else(LocalStoreFailure::open_boundary)?;
        let binding = SemanticEvent::for_context(
            context,
            source_id,
            "host-event-store-observation",
            0,
            0,
            "observe.store",
            "unknown",
        )
        .map_err(|_| LocalStoreFailure::binding())?;
        let state = match HostState::open_existing_for_target(home, target) {
            Ok(state) => Some(state),
            Err(super::routine::HostFailure::Unavailable) => None,
            Err(_) => return Err(LocalStoreFailure::open_boundary()),
        };
        let store = match state.as_ref() {
            Some(state) => state
                .open_event_store_existing(target, context, source_id)
                .map_err(|_| LocalStoreFailure::open_boundary())?,
            None => None,
        };
        Ok(Self {
            home: home.to_path_buf(),
            target: target.to_path_buf(),
            context_id: binding.context_id().to_owned(),
            candidate_id: binding.candidate_id().to_owned(),
            source_id: binding.source_id().to_owned(),
            state,
            store,
        })
    }

    pub(super) fn status(&self) -> &'static str {
        if self.store.is_some() {
            "available"
        } else {
            "absent"
        }
    }

    pub(super) fn query_diagnostic(
        &self,
        query: &EventQuery,
    ) -> Result<Vec<SemanticEvent>, LocalStoreFailure> {
        match &self.store {
            Some(store) => store
                .query(query)
                .map_err(|error| LocalStoreFailure::query(&error)),
            None => Ok(Vec::new()),
        }
    }

    pub(super) fn explain(
        &self,
        query: &EventQuery,
        event_id: &str,
    ) -> Result<Option<CausalExplanation>, LocalStoreFailure> {
        match &self.store {
            Some(store) => store
                .explain(query, event_id)
                .map(Some)
                .map_err(|error| LocalStoreFailure::explain(&error)),
            None => Ok(None),
        }
    }

    pub(super) fn revalidate(&self) -> Result<(), LocalStoreFailure> {
        match (&self.state, &self.store) {
            (Some(state), Some(store)) => state
                .verify_event_store(store)
                .map_err(|_| LocalStoreFailure::changed()),
            (Some(state), None) => {
                if state
                    .event_store_absent(
                        &self.target,
                        &self.context_id,
                        &self.candidate_id,
                        &self.source_id,
                    )
                    .unwrap_or(false)
                {
                    Ok(())
                } else {
                    Err(LocalStoreFailure::changed())
                }
            }
            (None, None) => match HostState::open_existing_for_target(&self.home, &self.target) {
                Err(super::routine::HostFailure::Unavailable) => Ok(()),
                _ => Err(LocalStoreFailure::changed()),
            },
            _ => Err(LocalStoreFailure::changed()),
        }
    }
}

pub(super) fn local_policy() -> Value {
    json!({
        "schema_version": "LocalObservabilityPolicy-v1",
        "mode": "local-only",
        "store_limit_bytes": EventStore::supported_store_limit_bytes(),
        "event_limit": EventStore::supported_event_limit(),
        "scan_row_limit": EventStore::supported_scan_limit(),
        "query_result_limit": EventStore::supported_result_limit(),
        "lock_timeout_millis": EventStore::supported_lock_timeout_millis(),
        "deletion": "explicit-clear-api",
        "external_export": "disabled-safe-default-OD-004-OD-007",
        "configured_export_on_read": "refused-no-external-effect",
        "export_requires": "explicit-config-consent-authorized-external-write-roundtrip"
    })
}
