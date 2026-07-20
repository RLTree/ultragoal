use crate::context::LiveContext;
use crate::observability::{CausalExplanation, EventQuery, EventStore, SemanticEvent};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const STORE_PATH: &str = "validation_artifacts/observability/spool/successor-events.jsonl";
pub(super) const RUNTIME_SOURCE_ID: &str = "successor-runtime";

mod failure;
mod terminal_routine;
#[cfg(test)]
mod terminal_routine_tests;
pub(super) use failure::LocalStoreFailure;
pub(super) use terminal_routine::routine_observations_from_events;
#[allow(unused_imports)]
pub(super) use terminal_routine::{
    RoutineTerminalEvent, append_routine_terminal, terminal_event_id,
};

pub(super) struct LocalStore {
    root: PathBuf,
    before: StoreState,
    store: Option<EventStore>,
}

impl LocalStore {
    pub(super) fn open(
        root: &Path,
        context: &LiveContext,
        source_id: &str,
    ) -> Result<Self, LocalStoreFailure> {
        let path = store_path(root);
        let before = store_state(root, &path).ok_or_else(LocalStoreFailure::open_boundary)?;
        let store = match before.leaf {
            LeafState::Absent { .. } => None,
            LeafState::Present { .. } => Some(
                EventStore::for_context(path, context, source_id)
                    .map_err(|error| LocalStoreFailure::open(&error))?,
            ),
        };
        Ok(Self {
            root: root.to_path_buf(),
            before,
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
        match store_state(&self.root, &store_path(&self.root)) {
            Some(state) if state == self.before => Ok(()),
            _ => Err(LocalStoreFailure::changed()),
        }
    }
}

pub(super) fn store_path(root: &Path) -> PathBuf {
    root.join(STORE_PATH)
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoreState {
    directories: Vec<PathIdentity>,
    leaf: LeafState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PathIdentity {
    name: OsString,
    device: u64,
    inode: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum LeafState {
    Absent {
        missing_component: usize,
    },
    Present {
        device: u64,
        inode: u64,
        byte_length: u64,
        modified_seconds: i64,
        modified_nanoseconds: i64,
        changed_seconds: i64,
        changed_nanoseconds: i64,
    },
}

#[cfg(unix)]
fn store_state(root: &Path, path: &Path) -> Option<StoreState> {
    let relative = path.strip_prefix(root).ok()?;
    let root_metadata = std::fs::symlink_metadata(root).ok()?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return None;
    }
    let mut directories = vec![PathIdentity {
        name: root.as_os_str().to_os_string(),
        device: root_metadata.dev(),
        inode: root_metadata.ino(),
    }];
    let components = relative.components().collect::<Vec<_>>();
    let mut current = root.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        let std::path::Component::Normal(name) = component else {
            return None;
        };
        current.push(name);
        let metadata = match std::fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Some(StoreState {
                    directories,
                    leaf: LeafState::Absent {
                        missing_component: index,
                    },
                });
            }
            Err(_) => return None,
        };
        if metadata.file_type().is_symlink() {
            return None;
        }
        if index + 1 == components.len() {
            if !metadata.is_file() || metadata.nlink() != 1 {
                return None;
            }
            return Some(StoreState {
                directories,
                leaf: LeafState::Present {
                    device: metadata.dev(),
                    inode: metadata.ino(),
                    byte_length: metadata.len(),
                    modified_seconds: metadata.mtime(),
                    modified_nanoseconds: metadata.mtime_nsec(),
                    changed_seconds: metadata.ctime(),
                    changed_nanoseconds: metadata.ctime_nsec(),
                },
            });
        }
        if !metadata.is_dir() {
            return None;
        }
        directories.push(PathIdentity {
            name: name.to_os_string(),
            device: metadata.dev(),
            inode: metadata.ino(),
        });
    }
    None
}

#[cfg(not(unix))]
fn store_state(_: &Path, _: &Path) -> Option<StoreState> {
    None
}
