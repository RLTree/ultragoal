use crate::context::LiveContext;
use crate::observability::{CausalExplanation, EventQuery, EventStore, SemanticEvent};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const STORE_PATH: &str = "validation_artifacts/observability/spool/successor-events.jsonl";

pub(super) struct LocalStore {
    root: PathBuf,
    before: StoreState,
    store: Option<EventStore>,
}

impl LocalStore {
    pub(super) fn open(root: &Path, context: &LiveContext, source_id: &str) -> Result<Self, ()> {
        let path = store_path(root);
        let before = store_state(root, &path).ok_or(())?;
        let store = match before.leaf {
            LeafState::Absent { .. } => None,
            LeafState::Present { .. } => {
                Some(EventStore::for_context(path, context, source_id).map_err(|_| ())?)
            }
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

    pub(super) fn query(&self, query: &EventQuery) -> Result<Vec<SemanticEvent>, ()> {
        match &self.store {
            Some(store) => store.query(query).map_err(|_| ()),
            None => Ok(Vec::new()),
        }
    }

    pub(super) fn query_diagnostic(
        &self,
        query: &EventQuery,
    ) -> Result<Vec<SemanticEvent>, String> {
        match &self.store {
            Some(store) => store.query(query),
            None => Ok(Vec::new()),
        }
    }

    pub(super) fn explain(
        &self,
        query: &EventQuery,
        event_id: &str,
    ) -> Result<Option<CausalExplanation>, String> {
        match &self.store {
            Some(store) => store.explain(query, event_id).map(Some),
            None => Ok(None),
        }
    }

    pub(super) fn revalidate(&self) -> bool {
        store_state(&self.root, &store_path(&self.root)).is_some_and(|state| state == self.before)
    }
}

pub(super) fn store_path(root: &Path) -> PathBuf {
    root.join(STORE_PATH)
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
    Absent { missing_component: usize },
    Present { device: u64, inode: u64 },
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
            if !metadata.is_file() {
                return None;
            }
            return Some(StoreState {
                directories,
                leaf: LeafState::Present {
                    device: metadata.dev(),
                    inode: metadata.ino(),
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
