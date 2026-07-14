use super::*;

impl<E: RepositoryFitPermitEffects> FitEffects for ScopedEffects<E> {
    fn compare_exchange(
        &mut self,
        path: &CanonicalPath,
        expected: &ExpectedContent,
        replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        let allowed = self.allowed.iter().any(|row| {
            if &row.path != path {
                return false;
            }
            let forward = &row.forward_expected == expected
                && replacement == Some(row.replacement.as_slice());
            let rollback = *expected == ExpectedContent::ExactDigest(digest(&row.replacement))
                && replacement == row.prior.as_deref();
            forward || rollback
        });
        if !allowed {
            self.scope_violation = true;
            return Err(crate::repository_fit::error(FitErrorId::Unauthorized));
        }
        let before = self.capture_authorized()?;
        let row = self
            .allowed
            .iter()
            .find(|row| &row.path == path)
            .expect("the allowed predicate found this exact path")
            .clone();
        let forward =
            row.forward_expected == *expected && replacement == Some(row.replacement.as_slice());
        let rollback = *expected == ExpectedContent::ExactDigest(digest(&row.replacement))
            && replacement == row.prior.as_deref();
        let before_mode =
            target_object(&before.snapshot, path.as_str()).map(|object| object.mode & 0o7777);
        let expected_mode = match (forward, rollback) {
            (true, true) if before_mode == Some(row.forward_mode) => row.prior_mode,
            (true, _) => Some(row.forward_mode),
            (_, true) => row.prior_mode,
            _ => None,
        };
        let result = self.inner.compare_exchange(path, expected, replacement);
        let after = match capture_target_descriptor_chain_for_paths(&self.root, &self.target_paths)
        {
            Ok(after) => after,
            Err(_) => {
                self.chain_violation = true;
                return Err(chain_fit_error());
            }
        };
        match result {
            Ok(false) if after.snapshot == before.snapshot => {
                self.authorized_target = after;
                Ok(false)
            }
            Ok(true)
                if valid_target_effect_transition(
                    &before.snapshot,
                    &after.snapshot,
                    &self.target_prestate,
                    path,
                    replacement,
                    expected_mode,
                ) =>
            {
                self.authorized_target = after;
                Ok(true)
            }
            // An untrusted effect adapter may claim success without changing
            // the namespace. Preserve that report so the kernel's desired
            // state proof rejects it and causal reconciliation can still
            // prove the exact prior state rather than manufacturing ambiguity.
            Ok(true) if after.snapshot == before.snapshot => {
                self.authorized_target = after;
                Ok(true)
            }
            Err(failure) if after.snapshot == before.snapshot => {
                self.authorized_target = after;
                Err(failure)
            }
            Ok(_) | Err(_) => {
                self.chain_violation = true;
                Err(chain_fit_error())
            }
        }
    }
}

impl<E: RepositoryFitPermitEffects> RepositoryFitPermitEffects for ScopedEffects<E> {
    fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError> {
        self.inner.read_unix_mode(path)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct TargetSnapshot {
    pub(crate) sha256: String,
    pub(crate) rows: Vec<TargetRow>,
}

/// One internally consistent target observation. Every present component was
/// opened relative to its held parent descriptor and every absent boundary is
/// retained for an exact-name recheck. Keeping this value alive keeps the
/// complete root-to-leaf descriptor chains alive as well.
pub(crate) struct TargetCapture {
    pub(crate) snapshot: TargetSnapshot,
    pub(crate) chain: TargetDescriptorChain,
}

pub(crate) struct TargetDescriptorChain {
    pub(crate) root_path: PathBuf,
    pub(crate) root: File,
    pub(crate) root_object: ObjectRow,
    pub(crate) paths: Vec<HeldTargetPath>,
}

pub(crate) struct HeldTargetPath {
    pub(crate) attachments: Vec<HeldTargetAttachment>,
    pub(crate) missing: Option<HeldMissingAttachment>,
}

pub(crate) struct HeldTargetAttachment {
    pub(crate) path: String,
    pub(crate) parent: File,
    pub(crate) name: String,
    pub(crate) object: ObjectRow,
    pub(crate) opened: Option<File>,
}

pub(crate) struct HeldMissingAttachment {
    pub(crate) path: String,
    pub(crate) parent: File,
    pub(crate) name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExactEntry {
    Absent,
    Exact,
    Alias,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct TargetRow {
    pub(crate) path: String,
    pub(crate) state: TargetState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum TargetState {
    Missing,
    Present(ObjectRow),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ObjectRow {
    pub(crate) kind: &'static str,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) links: u64,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) mode: u32,
    pub(crate) byte_length: u64,
    pub(crate) payload_sha256: Option<String>,
    pub(crate) change_version: ProtectedChangeVersion,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagedAncestorContract {
    pub(crate) rows: Vec<ManagedAncestorContractRow>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagedAncestorContractRow {
    pub(crate) path: String,
    pub(crate) expectation: ManagedAncestorExpectation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ManagedAncestorExpectation {
    Existing {
        device: u64,
        inode: u64,
        uid: u32,
        gid: u32,
        mode: u32,
    },
    Missing,
}

pub(crate) struct RecoveryTargetContractObservation {
    pub(crate) root_binding: String,
    pub(crate) ancestor_preimage: bool,
    pub(crate) ancestor_postimage: bool,
    pub(crate) leaves: Vec<RecoveryLeafObservation>,
}
