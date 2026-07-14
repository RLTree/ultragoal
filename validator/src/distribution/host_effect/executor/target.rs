use super::*;
use crate::distribution::host_effect::lifecycle::{
    AcceptedHostScope, HostObjectIdentity, HostTargetObserver, ObservedTargetIdentity,
    PublicationClassification, SupportedHostLifecycleError, SupportedHostLifecycleErrorId,
    lifecycle_error,
};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

include!("target/fault.rs");

#[derive(Clone, Debug)]
pub(crate) struct ConfinedHostEffectTarget {
    root: PathBuf,
    scope: AcceptedHostScope,
    generation: u64,
    expected: ObservedTargetIdentity,
}

pub(crate) struct ConfinedHostEffectTargetObserver {
    target: ConfinedHostEffectTarget,
}

struct ConfinedHostEffectLease {
    identity: ObservedTargetIdentity,
    target: ConfinedHostEffectTarget,
}

pub(crate) struct PreparedPublication {
    name: String,
    temp_name: String,
    bytes: Vec<u8>,
    expectation: PublicationExpectation,
}

pub(crate) struct CommittedPublication {
    pub expectation: PublicationExpectation,
    pub observation: PublicationInventoryObservation,
}

pub(crate) struct PublicationFailure {
    pub id: HostEffectExecutorErrorId,
    pub observation: Option<PublicationInventoryObservation>,
}

impl PublicationFailure {
    fn new(id: HostEffectExecutorErrorId) -> Self {
        Self {
            id,
            observation: None,
        }
    }
    fn observed(
        id: HostEffectExecutorErrorId,
        observation: PublicationInventoryObservation,
    ) -> Self {
        Self {
            id,
            observation: Some(observation),
        }
    }
    pub(crate) const fn id(&self) -> HostEffectExecutorErrorId {
        self.id
    }
}

include!("target/bind.rs");

impl ConfinedHostEffectTarget {
    pub(crate) fn require_clean_publication_name(
        &self,
        name: &str,
    ) -> Result<(), PublicationFailure> {
        if unsafe_name(name) {
            return Err(PublicationFailure::new(
                HostEffectExecutorErrorId::InvalidTargetRoot,
            ));
        }
        if self.root.join(name).exists() {
            return Err(PublicationFailure::new(
                HostEffectExecutorErrorId::UnsafeObject,
            ));
        }
        if fs::read_dir(&self.root)
            .map_err(|_| PublicationFailure::new(HostEffectExecutorErrorId::InvalidTargetRoot))?
            .any(|row| {
                row.ok()
                    .and_then(|entry| entry.file_name().into_string().ok())
                    .is_some_and(|value| value.starts_with(&format!(".{name}.")))
            })
        {
            return Err(PublicationFailure::new(
                HostEffectExecutorErrorId::TempCollision,
            ));
        }
        Ok(())
    }

    pub(crate) fn prepare(
        &self,
        effect_identity_sha256: &str,
        name: &str,
        bytes: Vec<u8>,
    ) -> Result<PreparedPublication, PublicationFailure> {
        self.require_clean_publication_name(name)?;
        let temp_name = format!(".{name}.0000000000000000.tmp");
        let prior = observe_expected_missing(name)?;
        let content_sha256 = digest_bytes(&bytes);
        let next = expected_regular(name, bytes.len() as u64, &content_sha256)?;
        let temporary = expected_regular(&temp_name, bytes.len() as u64, &content_sha256)?;
        let expectation =
            PublicationExpectation::new(effect_identity_sha256.into(), prior, next, temporary)
                .map_err(|_| {
                    PublicationFailure::new(HostEffectExecutorErrorId::FalsePassReceipt)
                })?;
        Ok(PreparedPublication {
            name: name.into(),
            temp_name,
            bytes,
            expectation,
        })
    }

    pub(crate) fn publish(
        &self,
        prepared: PreparedPublication,
        head: &HostEffectLedgerHead,
    ) -> Result<CommittedPublication, PublicationFailure> {
        let temp = self.root.join(&prepared.temp_name);
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|_| PublicationFailure::new(HostEffectExecutorErrorId::TempCollision))?;
        if run_fault(FaultPoint::BeforeTempFsync, &self.root) {
            let observation = self.observe(&prepared.expectation, head, None)?;
            return Err(PublicationFailure::observed(
                HostEffectExecutorErrorId::SyncFailure,
                observation,
            ));
        }
        file.write_all(&prepared.bytes)
            .map_err(|_| PublicationFailure::new(HostEffectExecutorErrorId::SyncFailure))?;
        file.sync_all()
            .map_err(|_| PublicationFailure::new(HostEffectExecutorErrorId::SyncFailure))?;
        fs::set_permissions(&temp, fs::Permissions::from_mode(0o400))
            .map_err(|_| PublicationFailure::new(HostEffectExecutorErrorId::SyncFailure))?;
        let rename_fault = run_fault(FaultPoint::BeforeRename, &self.root);
        let final_path = self.root.join(&prepared.name);
        if final_path.exists() || rename_fault {
            let observation = self.observe(&prepared.expectation, head, None)?;
            return Err(PublicationFailure::observed(
                HostEffectExecutorErrorId::RenameRace,
                observation,
            ));
        }
        fs::rename(&temp, &final_path)
            .map_err(|_| PublicationFailure::new(HostEffectExecutorErrorId::RenameRace))?;
        if run_fault(FaultPoint::BeforeDirectoryFsync, &self.root) {
            let observation = self.observe(&prepared.expectation, head, None)?;
            return Err(PublicationFailure::observed(
                HostEffectExecutorErrorId::SyncFailure,
                observation,
            ));
        }
        let observation = self.observe(&prepared.expectation, head, None)?;
        let classification = observation
            .classify()
            .map_err(|_| PublicationFailure::new(HostEffectExecutorErrorId::FalsePassReceipt))?;
        if classification.id() != PublicationClassificationId::CommittedBeforeAcknowledgement {
            return Err(PublicationFailure {
                id: HostEffectExecutorErrorId::FalsePassReceipt,
                observation: Some(observation),
            });
        }
        Ok(CommittedPublication {
            expectation: prepared.expectation,
            observation,
        })
    }

    pub(crate) fn acknowledge(
        &self,
        committed: &CommittedPublication,
        acknowledgement: PublicationAcknowledgementIdentity,
        head: HostEffectLedgerHead,
    ) -> Result<(PublicationInventoryObservation, PublicationClassification), PublicationFailure>
    {
        let observation = self.observe(&committed.expectation, &head, Some(acknowledgement))?;
        let classification = observation
            .classify()
            .map_err(|_| PublicationFailure::new(HostEffectExecutorErrorId::FalsePassReceipt))?;
        Ok((observation, classification))
    }

    pub(crate) fn reobserve_committed(
        &self,
        committed: &CommittedPublication,
        head: HostEffectLedgerHead,
    ) -> Result<(PublicationInventoryObservation, PublicationClassification), PublicationFailure>
    {
        let observation = self.observe(&committed.expectation, &head, None)?;
        let classification = observation
            .classify()
            .map_err(|_| PublicationFailure::new(HostEffectExecutorErrorId::FalsePassReceipt))?;
        Ok((observation, classification))
    }

    pub(crate) fn reobserve_failure(
        &self,
        failure: &PublicationFailure,
        _head: HostEffectLedgerHead,
    ) -> Result<(PublicationInventoryObservation, PublicationClassification), PublicationFailure>
    {
        if !self.root.is_dir() {
            return Err(PublicationFailure::new(HostEffectExecutorErrorId::PathSwap));
        }
        let observation = failure
            .observation
            .clone()
            .ok_or_else(|| PublicationFailure::new(HostEffectExecutorErrorId::FalsePassReceipt))?;
        let classification = observation
            .classify()
            .map_err(|_| PublicationFailure::new(HostEffectExecutorErrorId::FalsePassReceipt))?;
        Ok((observation, classification))
    }

    fn identity(&self) -> Result<ObservedTargetIdentity, SupportedHostLifecycleError> {
        let object = HostObjectIdentity::from_metadata(
            &fs::symlink_metadata(&self.root)
                .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::TargetSubstitution))?,
        )?;
        ObservedTargetIdentity::new(&self.scope, self.generation, object)
    }
}

include!("target/observation_method.rs");

include!("target/lease.rs");

include!("target/observation.rs");
