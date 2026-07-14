use super::*;

pub(crate) struct RecoveryLeafObservation {
    pub(crate) path: String,
    pub(crate) payload_sha256: Option<String>,
    pub(crate) mode: Option<u32>,
    pub(crate) valid_managed_leaf: bool,
}

impl ManagedAncestorContract {
    pub(crate) fn valid_for_leaf_paths(&self, leaf_paths: &[String]) -> bool {
        if self.rows.is_empty() || self.rows.len() > MAX_MANAGED_ANCESTOR_CONTRACT_ROWS {
            return false;
        }
        let targets = match leaf_paths
            .iter()
            .map(|path| CanonicalPath::parse(path))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(targets) => targets,
            Err(_) => return false,
        };
        let expected_paths = managed_ancestor_paths_for_targets(&targets);
        if self
            .rows
            .iter()
            .map(|row| row.path.as_str())
            .ne(expected_paths.iter().map(String::as_str))
        {
            return false;
        }
        self.rows.iter().enumerate().all(|(index, row)| {
            if index == 0 && row.path != "" {
                return false;
            }
            match row.expectation {
                ManagedAncestorExpectation::Existing {
                    device,
                    inode,
                    mode,
                    ..
                } => device != 0 && inode != 0 && mode & 0o170000 == 0o040000,
                ManagedAncestorExpectation::Missing => !row.path.is_empty(),
            }
        })
    }
}

#[cfg(test)]
pub(crate) fn managed_ancestor_contract_for_ledger_test() -> ManagedAncestorContract {
    ManagedAncestorContract {
        rows: vec![ManagedAncestorContractRow {
            path: String::new(),
            expectation: ManagedAncestorExpectation::Existing {
                device: 1,
                inode: 1,
                uid: 0,
                gid: 0,
                mode: 0o040755,
            },
        }],
    }
}

impl ObjectRow {
    pub(crate) fn managed_ancestor_equivalent(&self, other: &Self) -> bool {
        self.kind == "directory"
            && other.kind == "directory"
            && self.device == other.device
            && self.inode == other.inode
            && self.uid == other.uid
            && self.gid == other.gid
            && self.mode == other.mode
    }

    pub(crate) fn valid_created_managed_ancestor(&self, root: &Self) -> bool {
        self.kind == "directory"
            && self.device == root.device
            && self.uid == root.uid
            && self.gid == root.gid
            && self.mode & 0o7777 == CREATED_MANAGED_ANCESTOR_MODE
    }

    pub(crate) fn rollback_equivalent(&self, other: &Self, leaf: bool) -> bool {
        self.kind == other.kind
            && self.device == other.device
            && (self.inode == other.inode || (leaf && self.kind == "regular"))
            && self.links == other.links
            && self.uid == other.uid
            && self.gid == other.gid
            && self.mode == other.mode
            && self.byte_length == other.byte_length
            && self.payload_sha256 == other.payload_sha256
    }
}

pub(crate) fn target_object<'a>(snapshot: &'a TargetSnapshot, path: &str) -> Option<&'a ObjectRow> {
    snapshot
        .rows
        .iter()
        .find(|row| row.path == path)
        .and_then(|row| match &row.state {
            TargetState::Present(object) => Some(object),
            TargetState::Missing => None,
        })
}

pub(crate) fn valid_target_effect_transition(
    before: &TargetSnapshot,
    after: &TargetSnapshot,
    permit_prestate: &TargetSnapshot,
    mutation_path: &CanonicalPath,
    replacement: Option<&[u8]>,
    expected_mode: Option<u32>,
) -> bool {
    let before_rows = before
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let after_rows = after
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    let permit_rows = permit_prestate
        .rows
        .iter()
        .map(|row| (row.path.as_str(), &row.state))
        .collect::<BTreeMap<_, _>>();
    if before_rows.keys().ne(after_rows.keys()) || before_rows.keys().ne(permit_rows.keys()) {
        return false;
    }
    let root = match present_target_row(&after_rows, "") {
        Some(root) => root,
        None => return false,
    };
    let target = mutation_path.as_str();
    for (path, before_state) in before_rows {
        let after_state = after_rows[path];
        if path == target {
            let valid_leaf = match (replacement, expected_mode, after_state) {
                (None, _, TargetState::Missing) => true,
                (Some(bytes), Some(mode), TargetState::Present(object)) => {
                    object.kind == "regular"
                        && object.device == root.device
                        && object.links == 1
                        && object.uid == root.uid
                        && object.gid == root.gid
                        && object.mode & 0o7777 == mode
                        && object.byte_length == bytes.len() as u64
                        && object.payload_sha256.as_deref() == Some(digest(bytes).as_str())
                }
                _ => false,
            };
            if !valid_leaf {
                return false;
            }
            continue;
        }
        let strict_ancestor = path.is_empty()
            || (target.len() > path.len()
                && target.starts_with(path)
                && target.as_bytes().get(path.len()) == Some(&b'/'));
        if !strict_ancestor {
            if before_state != after_state {
                return false;
            }
            continue;
        }
        let valid_ancestor = match (before_state, after_state, permit_rows[path]) {
            (TargetState::Present(before), TargetState::Present(after), _) => {
                before.managed_ancestor_equivalent(after)
            }
            (TargetState::Missing, TargetState::Present(after), TargetState::Missing) => {
                after.valid_created_managed_ancestor(root)
            }
            (TargetState::Present(_), TargetState::Missing, TargetState::Missing) => true,
            (TargetState::Missing, TargetState::Missing, _) => true,
            _ => false,
        };
        if !valid_ancestor {
            return false;
        }
    }
    true
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ProtectedSnapshot {
    pub(crate) sha256: String,
    pub(crate) rows: Vec<ProtectedRow>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ProtectedRow {
    pub(crate) path: Vec<u8>,
    pub(crate) object: ObjectRow,
    pub(crate) change_version: ProtectedChangeVersion,
}
