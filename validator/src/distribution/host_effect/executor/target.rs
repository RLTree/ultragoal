use super::super::lifecycle::{
    AcceptedHostScope, ExpectedPublicationObjectIdentity, HostObjectIdentity, HostTargetLease,
    HostTargetObserver, ObservedTargetIdentity, PublicationAcknowledgementIdentity,
    PublicationClassification, PublicationExpectation, PublicationInventoryObservation,
    PublicationObjectKind, PublicationObjectObservation, SupportedHostLifecycleError,
    SupportedHostLifecycleErrorId, lifecycle_error,
};
use super::model::{HostEffectExecutorErrorId, HostEffectExecutorFailure, digest_bytes};
use std::ffi::{CStr, CString};
use std::fs::{self, File, Metadata};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const ROOT_PREFIX: &str = "hul-supported-host-effect-";
const MAX_PUBLICATION_BYTES: usize = 1024 * 1024;
const PUBLICATION_MODE: u32 = 0o400;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StableDirectoryIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
}

impl StableDirectoryIdentity {
    fn capture(metadata: &Metadata) -> Result<Self, HostEffectExecutorFailure> {
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.mode() & 0o777 != 0o700
            || metadata.uid() != unsafe { libc::geteuid() }
        {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::InvalidTargetRoot,
            ));
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
        })
    }
}

struct TargetRootAnchor {
    canonical_path: PathBuf,
    parent: File,
    directory: File,
    name: CString,
    identity: StableDirectoryIdentity,
    scope: AcceptedHostScope,
    target_generation: u64,
    expected_target: ObservedTargetIdentity,
}

#[derive(Clone)]
pub(crate) struct ConfinedHostEffectTarget {
    anchor: Arc<TargetRootAnchor>,
}

impl std::fmt::Debug for ConfinedHostEffectTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConfinedHostEffectTarget")
            .field("canonical_path", &self.anchor.canonical_path)
            .field("target_generation", &self.anchor.target_generation)
            .finish_non_exhaustive()
    }
}

impl ConfinedHostEffectTarget {
    /// Binds one dedicated target root below `/private/tmp`. Opening and
    /// observing the root performs no mutation.
    pub(in crate::distribution::host_effect) fn bind(
        path: &Path,
        scope: AcceptedHostScope,
        target_generation: u64,
    ) -> Result<(Self, ObservedTargetIdentity), HostEffectExecutorFailure> {
        if target_generation == 0 {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::InvalidTargetRoot,
            ));
        }
        let private_tmp = fs::canonicalize("/private/tmp").map_err(|_| io_failure())?;
        let canonical = fs::canonicalize(path).map_err(|_| io_failure())?;
        let name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| value.starts_with(ROOT_PREFIX))
            .ok_or_else(|| {
                HostEffectExecutorFailure::new(HostEffectExecutorErrorId::InvalidTargetRoot)
            })?;
        if path != canonical
            || canonical.parent() != Some(private_tmp.as_path())
            || !valid_component(name)
        {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::InvalidTargetRoot,
            ));
        }
        let parent = open_directory_path(&private_tmp)?;
        let name = CString::new(name).map_err(|_| io_failure())?;
        let descriptor = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if descriptor < 0 {
            return Err(io_failure());
        }
        let directory = unsafe { File::from_raw_fd(descriptor) };
        let descriptor_metadata = directory.metadata().map_err(|_| io_failure())?;
        let named_metadata = fs::symlink_metadata(&canonical).map_err(|_| io_failure())?;
        let identity = StableDirectoryIdentity::capture(&descriptor_metadata)?;
        if StableDirectoryIdentity::capture(&named_metadata)? != identity {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::TargetSubstitution,
            ));
        }
        let object = HostObjectIdentity::from_metadata(&descriptor_metadata)
            .map_err(|_| target_substitution())?;
        let expected_target = ObservedTargetIdentity::new(&scope, target_generation, object)
            .map_err(|_| target_substitution())?;
        let target = Self {
            anchor: Arc::new(TargetRootAnchor {
                canonical_path: canonical,
                parent,
                directory,
                name,
                identity,
                scope,
                target_generation,
                expected_target: expected_target.clone(),
            }),
        };
        target.revalidate_anchor()?;
        Ok((target, expected_target))
    }

    pub(in crate::distribution::host_effect) fn observer(
        &self,
    ) -> ConfinedHostEffectTargetObserver {
        ConfinedHostEffectTargetObserver {
            target: self.clone(),
        }
    }

    pub(super) fn expected_target(&self) -> &ObservedTargetIdentity {
        &self.anchor.expected_target
    }

    pub(super) fn revalidate_anchor(&self) -> Result<(), HostEffectExecutorFailure> {
        let descriptor = self.anchor.directory.metadata().map_err(|_| path_swap())?;
        if StableDirectoryIdentity::capture(&descriptor)? != self.anchor.identity {
            return Err(path_swap());
        }
        let named = stat_at(
            self.anchor.parent.as_raw_fd(),
            &self.anchor.name,
            libc::AT_SYMLINK_NOFOLLOW,
        )?
        .ok_or_else(path_swap)?;
        if StableDirectoryIdentity::capture(&named)? != self.anchor.identity {
            return Err(path_swap());
        }
        Ok(())
    }

    fn current_target_identity(&self) -> Result<ObservedTargetIdentity, HostEffectExecutorFailure> {
        self.revalidate_anchor()?;
        let metadata = self.anchor.directory.metadata().map_err(|_| path_swap())?;
        let object = HostObjectIdentity::from_metadata(&metadata).map_err(|_| path_swap())?;
        ObservedTargetIdentity::new(&self.anchor.scope, self.anchor.target_generation, object)
            .map_err(|_| target_substitution())
    }

    pub(super) fn prepare(
        &self,
        effect_identity_sha256: &str,
        target_name: &str,
        bytes: Vec<u8>,
    ) -> Result<PreparedPublication, HostEffectExecutorFailure> {
        if !valid_component(target_name)
            || !target_name.ends_with(".json")
            || bytes.is_empty()
            || bytes.len() > MAX_PUBLICATION_BYTES
        {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::UnsafeObject,
            ));
        }
        self.require_clean_publication_name(target_name)?;
        let target = self.observe_object(target_name, 1, false)?;
        if target.kind != PublicationObjectKind::Missing {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::Replay,
            ));
        }
        let mut nonce = [0_u8; 8];
        getrandom::fill(&mut nonce).map_err(|_| io_failure())?;
        let temporary_name = format!(
            ".{target_name}.{}.tmp",
            nonce
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        nonce.fill(0);
        if self.observe_object(&temporary_name, 1, false)?.kind != PublicationObjectKind::Missing {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::TempCollision,
            ));
        }
        let content_sha256 = digest_bytes(&bytes);
        let mode = u32::from(libc::S_IFREG) | PUBLICATION_MODE;
        let prior = ExpectedPublicationObjectIdentity::missing(target_name.to_owned())
            .map_err(|_| unsafe_object())?;
        let next = ExpectedPublicationObjectIdentity::regular(
            target_name.to_owned(),
            bytes.len() as u64,
            mode,
            1,
            content_sha256.clone(),
            true,
        )
        .map_err(|_| unsafe_object())?;
        let temporary = ExpectedPublicationObjectIdentity::regular(
            temporary_name.clone(),
            bytes.len() as u64,
            mode,
            1,
            content_sha256,
            true,
        )
        .map_err(|_| unsafe_object())?;
        let expectation =
            PublicationExpectation::new(effect_identity_sha256.to_owned(), prior, next, temporary)
                .map_err(|_| unsafe_object())?;
        Ok(PreparedPublication {
            target_name: target_name.to_owned(),
            temporary_name,
            bytes,
            expectation,
        })
    }

    pub(super) fn require_clean_publication_name(
        &self,
        target_name: &str,
    ) -> Result<(), HostEffectExecutorFailure> {
        if !valid_component(target_name) || !target_name.ends_with(".json") {
            return Err(unsafe_object());
        }
        self.revalidate_anchor()?;
        let names = self.names()?;
        let temporary_prefix = format!(".{target_name}.");
        for name in names.iter().filter(|name| {
            name.as_str() == target_name
                || (name.starts_with(&temporary_prefix) && name.ends_with(".tmp"))
        }) {
            let observed = self.observe_object(name, 1, false)?;
            if observed.kind != PublicationObjectKind::Regular {
                return Err(unsafe_object());
            }
            return Err(HostEffectExecutorFailure::new(if name == target_name {
                HostEffectExecutorErrorId::Replay
            } else {
                HostEffectExecutorErrorId::TempCollision
            }));
        }
        Ok(())
    }

    pub(super) fn publish(
        &self,
        prepared: PreparedPublication,
        current_ledger_head: &super::super::HostEffectLedgerHead,
    ) -> Result<CommittedPublication, PublicationFailure> {
        match self.publish_inner(&prepared, current_ledger_head) {
            Ok(observation) => Ok(CommittedPublication {
                target_name: prepared.target_name,
                temporary_name: prepared.temporary_name,
                expectation: prepared.expectation,
                observation,
            }),
            Err(id) => {
                let observed = self.observe_inventory(
                    &prepared.target_name,
                    &prepared.temporary_name,
                    prepared.expectation.clone(),
                    current_ledger_head.clone(),
                    None,
                    false,
                );
                match observed {
                    Ok((observation, classification)) => Err(PublicationFailure {
                        id,
                        target_name: prepared.target_name,
                        temporary_name: prepared.temporary_name,
                        expectation: prepared.expectation,
                        observation: Some(observation),
                        classification: Some(classification),
                    }),
                    Err(_) => Err(PublicationFailure {
                        id,
                        target_name: prepared.target_name,
                        temporary_name: prepared.temporary_name,
                        expectation: prepared.expectation,
                        observation: None,
                        classification: None,
                    }),
                }
            }
        }
    }

    fn publish_inner(
        &self,
        prepared: &PreparedPublication,
        current_ledger_head: &super::super::HostEffectLedgerHead,
    ) -> Result<PublicationInventoryObservation, HostEffectExecutorErrorId> {
        self.revalidate_anchor().map_err(|failure| failure.id())?;
        if self
            .observe_object(&prepared.target_name, 1, false)
            .map_err(|failure| failure.id())?
            .kind
            != PublicationObjectKind::Missing
        {
            return Err(HostEffectExecutorErrorId::RenameRace);
        }
        run_fault(FaultPoint::BeforeTempCreate, &self.anchor.canonical_path)
            .map_err(|_| HostEffectExecutorErrorId::Io)?;
        let temporary = CString::new(prepared.temporary_name.as_str())
            .map_err(|_| HostEffectExecutorErrorId::UnsafeObject)?;
        let descriptor = unsafe {
            libc::openat(
                self.anchor.directory.as_raw_fd(),
                temporary.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                0o600,
            )
        };
        if descriptor < 0 {
            return Err(if last_errno() == Some(libc::EEXIST) {
                HostEffectExecutorErrorId::TempCollision
            } else {
                HostEffectExecutorErrorId::Io
            });
        }
        let mut file = unsafe { File::from_raw_fd(descriptor) };
        let initially_created =
            ObjectIdentity::capture(&file.metadata().map_err(|_| HostEffectExecutorErrorId::Io)?)
                .map_err(|_| HostEffectExecutorErrorId::UnsafeObject)?;
        if !initially_created.regular()
            || initially_created.links != 1
            || initially_created.device != self.anchor.identity.device
            || initially_created.uid != unsafe { libc::geteuid() }
        {
            return Err(HostEffectExecutorErrorId::UnsafeObject);
        }
        run_fault(FaultPoint::BeforeTempWrite, &self.anchor.canonical_path)
            .map_err(|_| HostEffectExecutorErrorId::Io)?;
        file.write_all(&prepared.bytes)
            .map_err(|_| HostEffectExecutorErrorId::Io)?;
        if unsafe { libc::fchmod(file.as_raw_fd(), PUBLICATION_MODE as libc::mode_t) } != 0 {
            return Err(HostEffectExecutorErrorId::Io);
        }
        run_fault(FaultPoint::BeforeTempFsync, &self.anchor.canonical_path)
            .map_err(|_| HostEffectExecutorErrorId::SyncFailure)?;
        file.sync_all()
            .map_err(|_| HostEffectExecutorErrorId::SyncFailure)?;
        let expected_temporary = self
            .read_regular(&prepared.temporary_name, MAX_PUBLICATION_BYTES)
            .map_err(|failure| failure.id())?
            .ok_or(HostEffectExecutorErrorId::RenameRace)?;
        if expected_temporary.bytes != prepared.bytes
            || expected_temporary.identity.device != initially_created.device
            || expected_temporary.identity.inode != initially_created.inode
            || expected_temporary.identity.mode & 0o777 != PUBLICATION_MODE
        {
            return Err(HostEffectExecutorErrorId::RenameRace);
        }
        let created = expected_temporary.identity;
        run_fault(FaultPoint::BeforeRename, &self.anchor.canonical_path)
            .map_err(|_| HostEffectExecutorErrorId::RenameRace)?;
        self.revalidate_anchor()
            .map_err(|_| HostEffectExecutorErrorId::PathSwap)?;
        if self
            .observe_object(&prepared.target_name, 1, false)
            .map_err(|failure| failure.id())?
            .kind
            != PublicationObjectKind::Missing
        {
            return Err(HostEffectExecutorErrorId::RenameRace);
        }
        if !self
            .read_regular(&prepared.temporary_name, MAX_PUBLICATION_BYTES)
            .map_err(|failure| failure.id())?
            .is_some_and(|current| {
                current.bytes == prepared.bytes
                    && current.identity.device == created.device
                    && current.identity.inode == created.inode
                    && current.identity.links == 1
            })
        {
            return Err(HostEffectExecutorErrorId::RenameRace);
        }
        let target = CString::new(prepared.target_name.as_str())
            .map_err(|_| HostEffectExecutorErrorId::UnsafeObject)?;
        if rename_noreplace(self.anchor.directory.as_raw_fd(), &temporary, &target) != 0 {
            return Err(match last_errno() {
                Some(libc::EEXIST) | Some(libc::ENOENT) => HostEffectExecutorErrorId::RenameRace,
                _ => HostEffectExecutorErrorId::Io,
            });
        }
        let committed = self
            .read_regular(&prepared.target_name, MAX_PUBLICATION_BYTES)
            .map_err(|failure| failure.id())?
            .ok_or(HostEffectExecutorErrorId::RenameRace)?;
        if committed.bytes != prepared.bytes
            || !committed.identity.same_after_rename(created)
            || self
                .observe_object(&prepared.temporary_name, 1, false)
                .map_err(|failure| failure.id())?
                .kind
                != PublicationObjectKind::Missing
        {
            return Err(HostEffectExecutorErrorId::RenameRace);
        }
        run_fault(
            FaultPoint::BeforeDirectoryFsync,
            &self.anchor.canonical_path,
        )
        .map_err(|_| HostEffectExecutorErrorId::SyncFailure)?;
        self.anchor
            .directory
            .sync_all()
            .map_err(|_| HostEffectExecutorErrorId::SyncFailure)?;
        let (observation, classification) = self
            .observe_inventory(
                &prepared.target_name,
                &prepared.temporary_name,
                prepared.expectation.clone(),
                current_ledger_head.clone(),
                None,
                true,
            )
            .map_err(|failure| failure.id())?;
        if classification.id()
            != super::super::lifecycle::PublicationClassificationId::CommittedBeforeAcknowledgement
        {
            return Err(HostEffectExecutorErrorId::FalsePassReceipt);
        }
        Ok(observation)
    }

    pub(super) fn acknowledge(
        &self,
        committed: &CommittedPublication,
        acknowledgement: PublicationAcknowledgementIdentity,
        ledger_head: super::super::HostEffectLedgerHead,
    ) -> Result<
        (PublicationInventoryObservation, PublicationClassification),
        HostEffectExecutorFailure,
    > {
        self.observe_inventory(
            &committed.target_name,
            &committed.temporary_name,
            committed.expectation.clone(),
            ledger_head,
            Some(acknowledgement),
            true,
        )
    }

    pub(super) fn reobserve_committed(
        &self,
        committed: &CommittedPublication,
        ledger_head: super::super::HostEffectLedgerHead,
    ) -> Result<
        (PublicationInventoryObservation, PublicationClassification),
        HostEffectExecutorFailure,
    > {
        self.observe_inventory(
            &committed.target_name,
            &committed.temporary_name,
            committed.expectation.clone(),
            ledger_head,
            None,
            true,
        )
    }

    pub(super) fn reobserve_failure(
        &self,
        failure: &PublicationFailure,
        ledger_head: super::super::HostEffectLedgerHead,
    ) -> Result<
        (PublicationInventoryObservation, PublicationClassification),
        HostEffectExecutorFailure,
    > {
        self.observe_inventory(
            &failure.target_name,
            &failure.temporary_name,
            failure.expectation.clone(),
            ledger_head,
            None,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn observe_inventory(
        &self,
        target_name: &str,
        expected_temporary_name: &str,
        expectation: PublicationExpectation,
        ledger_head: super::super::HostEffectLedgerHead,
        acknowledgement: Option<PublicationAcknowledgementIdentity>,
        data_synced: bool,
    ) -> Result<
        (PublicationInventoryObservation, PublicationClassification),
        HostEffectExecutorFailure,
    > {
        self.revalidate_anchor()?;
        let before =
            ObjectIdentity::capture(&self.anchor.directory.metadata().map_err(|_| path_swap())?)?;
        let scan_generation = observation_generation(&before);
        let target = self
            .observe_object(target_name, scan_generation, data_synced)?
            .observation;
        let temporary_prefix = format!(".{target_name}.");
        let mut temporary_objects = Vec::new();
        for name in self.names()? {
            if name.starts_with(&temporary_prefix) && name.ends_with(".tmp") {
                temporary_objects.push(
                    self.observe_object(&name, scan_generation, data_synced)?
                        .observation,
                );
            }
        }
        // If the expected name disappeared but another collision appeared,
        // the complete matching inventory is still passed to classification.
        let _ = expected_temporary_name;
        let after =
            ObjectIdentity::capture(&self.anchor.directory.metadata().map_err(|_| path_swap())?)?;
        let after_generation = if before == after {
            scan_generation
        } else {
            scan_generation.saturating_add(1)
        };
        let observation = PublicationInventoryObservation::new(
            scan_generation,
            after_generation,
            target,
            temporary_objects,
            expectation,
            ledger_head,
            acknowledgement,
        )
        .map_err(|_| unsafe_object())?;
        let classification = observation.classify().map_err(|_| unsafe_object())?;
        Ok((observation, classification))
    }

    fn observe_object(
        &self,
        name: &str,
        generation: u64,
        data_synced: bool,
    ) -> Result<ObservedObject, HostEffectExecutorFailure> {
        let name_c = component(name)?;
        let Some(metadata) = stat_at(
            self.anchor.directory.as_raw_fd(),
            &name_c,
            libc::AT_SYMLINK_NOFOLLOW,
        )?
        else {
            return Ok(ObservedObject {
                kind: PublicationObjectKind::Missing,
                observation: PublicationObjectObservation::missing(name.to_owned(), generation)
                    .map_err(|_| unsafe_object())?,
            });
        };
        let identity = ObjectIdentity::capture(&metadata)?;
        let kind = identity.kind();
        if kind != PublicationObjectKind::Regular {
            return Ok(ObservedObject {
                kind,
                observation: PublicationObjectObservation::new(
                    name.to_owned(),
                    kind,
                    identity.length,
                    identity.mode,
                    identity.links,
                    None,
                    generation,
                    false,
                )
                .map_err(|_| unsafe_object())?,
            });
        }
        let snapshot = self
            .read_regular(name, MAX_PUBLICATION_BYTES)?
            .ok_or_else(target_substitution)?;
        Ok(ObservedObject {
            kind,
            observation: PublicationObjectObservation::new(
                name.to_owned(),
                kind,
                snapshot.bytes.len() as u64,
                snapshot.identity.mode,
                snapshot.identity.links,
                Some(digest_bytes(&snapshot.bytes)),
                generation,
                data_synced,
            )
            .map_err(|_| unsafe_object())?,
        })
    }

    fn read_regular(
        &self,
        name: &str,
        maximum: usize,
    ) -> Result<Option<FileSnapshot>, HostEffectExecutorFailure> {
        let name_c = component(name)?;
        let descriptor = unsafe {
            libc::openat(
                self.anchor.directory.as_raw_fd(),
                name_c.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return if last_errno() == Some(libc::ENOENT) {
                Ok(None)
            } else {
                Err(unsafe_object())
            };
        }
        let mut file = unsafe { File::from_raw_fd(descriptor) };
        let before = ObjectIdentity::capture(&file.metadata().map_err(|_| unsafe_object())?)?;
        if !before.regular()
            || before.links != 1
            || before.device != self.anchor.identity.device
            || before.uid != unsafe { libc::geteuid() }
            || before.length > maximum as u64
        {
            return Err(unsafe_object());
        }
        let mut bytes = Vec::with_capacity(before.length as usize);
        Read::by_ref(&mut file)
            .take(maximum as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| unsafe_object())?;
        let after = ObjectIdentity::capture(&file.metadata().map_err(|_| unsafe_object())?)?;
        let named = stat_at(
            self.anchor.directory.as_raw_fd(),
            &name_c,
            libc::AT_SYMLINK_NOFOLLOW,
        )?
        .ok_or_else(target_substitution)?;
        let named = ObjectIdentity::capture(&named)?;
        if bytes.len() > maximum
            || before != after
            || before != named
            || bytes.len() as u64 != before.length
        {
            return Err(target_substitution());
        }
        Ok(Some(FileSnapshot {
            identity: before,
            bytes,
        }))
    }

    fn names(&self) -> Result<Vec<String>, HostEffectExecutorFailure> {
        self.revalidate_anchor()?;
        let duplicate =
            unsafe { libc::fcntl(self.anchor.directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
        if duplicate < 0 {
            return Err(io_failure());
        }
        let directory = unsafe { libc::fdopendir(duplicate) };
        if directory.is_null() {
            unsafe { libc::close(duplicate) };
            return Err(io_failure());
        }
        let mut names = Vec::new();
        loop {
            clear_errno();
            let entry = unsafe { libc::readdir(directory) };
            if entry.is_null() {
                let read_error = last_errno();
                if read_error.is_some_and(|value| value != 0) {
                    unsafe { libc::closedir(directory) };
                    return Err(io_failure());
                }
                break;
            }
            let name = match unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_str() {
                Ok(name) => name,
                Err(_) => {
                    unsafe { libc::closedir(directory) };
                    return Err(unsafe_object());
                }
            };
            if !matches!(name, "." | "..") {
                if !valid_component(name) {
                    unsafe { libc::closedir(directory) };
                    return Err(unsafe_object());
                }
                names.push(name.to_owned());
            }
        }
        if unsafe { libc::closedir(directory) } != 0 {
            return Err(io_failure());
        }
        names.sort();
        names.dedup();
        self.revalidate_anchor()?;
        Ok(names)
    }
}

pub(crate) struct ConfinedHostEffectTargetObserver {
    target: ConfinedHostEffectTarget,
}

impl HostTargetObserver for ConfinedHostEffectTargetObserver {
    fn acquire(
        &mut self,
        expected: &ObservedTargetIdentity,
    ) -> Result<Box<dyn HostTargetLease>, SupportedHostLifecycleError> {
        let current = self
            .target
            .current_target_identity()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::TargetSubstitution))?;
        if &current != expected || expected != self.target.expected_target() {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::TargetSubstitution,
            ));
        }
        Ok(Box::new(ConfinedHostEffectTargetLease {
            target: self.target.clone(),
            identity: current,
        }))
    }
}

struct ConfinedHostEffectTargetLease {
    target: ConfinedHostEffectTarget,
    identity: ObservedTargetIdentity,
}

impl HostTargetLease for ConfinedHostEffectTargetLease {
    fn identity(&self) -> &ObservedTargetIdentity {
        &self.identity
    }

    fn revalidate(&mut self) -> Result<ObservedTargetIdentity, SupportedHostLifecycleError> {
        let current = self
            .target
            .current_target_identity()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::TargetSubstitution))?;
        if current != self.identity {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::TargetSubstitution,
            ));
        }
        Ok(current)
    }
}

pub(super) struct PreparedPublication {
    target_name: String,
    temporary_name: String,
    bytes: Vec<u8>,
    expectation: PublicationExpectation,
}

pub(super) struct CommittedPublication {
    pub(super) target_name: String,
    pub(super) temporary_name: String,
    pub(super) expectation: PublicationExpectation,
    pub(super) observation: PublicationInventoryObservation,
}

pub(super) struct PublicationFailure {
    pub(super) id: HostEffectExecutorErrorId,
    target_name: String,
    temporary_name: String,
    expectation: PublicationExpectation,
    pub(super) observation: Option<PublicationInventoryObservation>,
    pub(super) classification: Option<PublicationClassification>,
}

struct ObservedObject {
    kind: PublicationObjectKind,
    observation: PublicationObjectObservation,
}

struct FileSnapshot {
    identity: ObjectIdentity,
    bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ObjectIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl ObjectIdentity {
    fn capture(metadata: &Metadata) -> Result<Self, HostEffectExecutorFailure> {
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            links: metadata.nlink(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        })
    }

    fn regular(self) -> bool {
        self.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFREG)
    }

    fn kind(self) -> PublicationObjectKind {
        match self.mode & u32::from(libc::S_IFMT) {
            value if value == u32::from(libc::S_IFREG) => PublicationObjectKind::Regular,
            value if value == u32::from(libc::S_IFLNK) => PublicationObjectKind::Symlink,
            value if value == u32::from(libc::S_IFDIR) => PublicationObjectKind::Directory,
            value if value == u32::from(libc::S_IFIFO) => PublicationObjectKind::Fifo,
            value if value == u32::from(libc::S_IFSOCK) => PublicationObjectKind::Socket,
            value if value == u32::from(libc::S_IFCHR) || value == u32::from(libc::S_IFBLK) => {
                PublicationObjectKind::Device
            }
            _ => PublicationObjectKind::Unknown,
        }
    }

    fn same_after_rename(self, prior: Self) -> bool {
        self.device == prior.device
            && self.inode == prior.inode
            && self.mode == prior.mode
            && self.uid == prior.uid
            && self.gid == prior.gid
            && self.links == prior.links
            && self.length == prior.length
            && self.modified_seconds == prior.modified_seconds
            && self.modified_nanoseconds == prior.modified_nanoseconds
    }
}

fn observation_generation(identity: &ObjectIdentity) -> u64 {
    let mut value = identity.inode
        ^ identity.device.rotate_left(7)
        ^ (identity.changed_seconds.max(0) as u64).rotate_left(17)
        ^ (identity.changed_nanoseconds.max(0) as u64).rotate_left(29);
    if value == 0 {
        value = 1;
    }
    value
}

fn open_directory_path(path: &Path) -> Result<File, HostEffectExecutorFailure> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| io_failure())?;
    let descriptor = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if descriptor < 0 {
        return Err(io_failure());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn stat_at(
    directory: libc::c_int,
    name: &CStr,
    flags: libc::c_int,
) -> Result<Option<Metadata>, HostEffectExecutorFailure> {
    let descriptor = unsafe {
        libc::openat(
            directory,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor >= 0 {
        let file = unsafe { File::from_raw_fd(descriptor) };
        return file.metadata().map(Some).map_err(|_| io_failure());
    }
    if last_errno() == Some(libc::ENOENT) {
        return Ok(None);
    }
    // O_NOFOLLOW intentionally refuses symlinks. fstatat observes their kind
    // without following them so callers can classify rather than mistake the
    // refusal for absence.
    let mut value = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe { libc::fstatat(directory, name.as_ptr(), value.as_mut_ptr(), flags) };
    if result != 0 {
        return if last_errno() == Some(libc::ENOENT) {
            Ok(None)
        } else {
            Err(io_failure())
        };
    }
    let value = unsafe { value.assume_init() };
    // Rust cannot construct Metadata from stat. Re-opening special objects can
    // block or activate device behavior, so a successfully observed
    // non-openable object is conservatively rejected as unsafe.
    let _ = value;
    Err(unsafe_object())
}

fn component(value: &str) -> Result<CString, HostEffectExecutorFailure> {
    if !valid_component(value) {
        return Err(unsafe_object());
    }
    CString::new(value).map_err(|_| unsafe_object())
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !matches!(value, "." | "..")
        && value.is_ascii()
        && !value.bytes().any(|byte| {
            byte == b'/'
                || byte == b'\\'
                || byte == 0
                || byte.is_ascii_control()
                || !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
}

#[cfg(target_os = "macos")]
fn rename_noreplace(directory: libc::c_int, old: &CStr, new: &CStr) -> libc::c_int {
    const RENAME_NOFOLLOW_ANY: libc::c_uint = 0x10;
    const RENAME_RESOLVE_BENEATH: libc::c_uint = 0x20;
    unsafe {
        libc::renameatx_np(
            directory,
            old.as_ptr(),
            directory,
            new.as_ptr(),
            libc::RENAME_EXCL | RENAME_NOFOLLOW_ANY | RENAME_RESOLVE_BENEATH,
        )
    }
}

#[cfg(target_os = "linux")]
fn rename_noreplace(directory: libc::c_int, old: &CStr, new: &CStr) -> libc::c_int {
    unsafe {
        libc::renameat2(
            directory,
            old.as_ptr(),
            directory,
            new.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn rename_noreplace(_directory: libc::c_int, _old: &CStr, _new: &CStr) -> libc::c_int {
    -1
}

fn last_errno() -> Option<i32> {
    std::io::Error::last_os_error().raw_os_error()
}

#[cfg(target_os = "linux")]
fn clear_errno() {
    unsafe {
        *libc::__errno_location() = 0;
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
fn clear_errno() {
    unsafe {
        *libc::__error() = 0;
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
fn clear_errno() {}

fn io_failure() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Io)
}

fn path_swap() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::PathSwap)
}

fn target_substitution() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::TargetSubstitution)
}

fn unsafe_object() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::UnsafeObject)
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FaultPoint {
    BeforeTempCreate,
    BeforeTempWrite,
    BeforeTempFsync,
    BeforeRename,
    BeforeDirectoryFsync,
}

#[cfg(not(test))]
#[derive(Clone, Copy)]
enum FaultPoint {
    BeforeTempCreate,
    BeforeTempWrite,
    BeforeTempFsync,
    BeforeRename,
    BeforeDirectoryFsync,
}

#[cfg(test)]
thread_local! {
    static FAULT: std::cell::RefCell<Option<(FaultPoint, bool, Box<dyn FnOnce(&Path)>)>> =
        std::cell::RefCell::new(None);
}

#[cfg(test)]
pub(super) fn set_fault(
    point: FaultPoint,
    force_failure: bool,
    hook: impl FnOnce(&Path) + 'static,
) {
    FAULT.with(|slot| {
        assert!(
            slot.borrow_mut()
                .replace((point, force_failure, Box::new(hook)))
                .is_none(),
            "only one executor fault hook may be armed per thread"
        );
    });
}

#[cfg(test)]
fn run_fault(point: FaultPoint, path: &Path) -> Result<(), ()> {
    let armed = FAULT.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|(expected, _, _)| *expected == point)
        {
            slot.take()
        } else {
            None
        }
    });
    if let Some((_, fail, hook)) = armed {
        hook(path);
        if fail {
            return Err(());
        }
    }
    Ok(())
}

#[cfg(not(test))]
fn run_fault(_point: FaultPoint, _path: &Path) -> Result<(), ()> {
    Ok(())
}
