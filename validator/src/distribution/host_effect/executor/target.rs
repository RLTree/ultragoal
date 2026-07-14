use super::super::lifecycle::{
    AcceptedHostScope, ExpectedPublicationObjectIdentity, ExpectedRegularPublicationObject,
    HostObjectIdentity, HostTargetLease, HostTargetObserver, ObservedTargetIdentity,
    PublicationAcknowledgementIdentity, PublicationClassification, PublicationExpectation,
    PublicationInventoryObservation, PublicationInventoryObservationRequest, PublicationObjectKind,
    PublicationObjectObservation, PublicationObjectObservationRequest, SupportedHostLifecycleError,
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

include!("target_confinement/root_prefix.rs");

include!("target_confinement/binding.rs");

include!("target_confinement/name_validation.rs");

include!("target_confinement/publication.rs");

include!("target_confinement/reobservation_failure.rs");

include!("target_confinement/names.rs");

include!("target_confinement/observation_generation.rs");

include!("target_confinement/run_fault.rs");
