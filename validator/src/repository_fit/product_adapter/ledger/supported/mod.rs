use super::{
    AdapterErrorId, ExistingReservation, LedgerError, LedgerErrorId, RecoveryTargetSpec,
    RecoveryTerminal, RepositoryFitLedgerState, ReservationDecision, ReservationRequest,
    ReservationToken, canonical_recovery_intent_bytes,
};
use crate::repository_fit::{CanonicalPath, digest, valid_digest};
use getrandom::fill;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CStr, CString, OsStr};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[path = "object_identity.rs"]
mod object_identity;
#[path = "record_authentication.rs"]
mod record_authentication;
#[path = "replay_detection.rs"]
mod replay_detection;
#[path = "reservation_acquisition.rs"]
mod reservation_acquisition;
#[path = "reservation_identity.rs"]
mod reservation_identity;
#[path = "reservation_recovery.rs"]
mod reservation_recovery;
#[path = "snapshot_decoding.rs"]
mod snapshot_decoding;
#[path = "snapshot_transaction.rs"]
mod snapshot_transaction;
#[path = "store_create_exclusive.rs"]
mod store_create_exclusive;
#[path = "store_open.rs"]
mod store_open;
#[path = "transition_validation.rs"]
mod transition_validation;

pub(crate) use object_identity::*;
pub(crate) use record_authentication::*;
pub(crate) use replay_detection::*;
pub(crate) use reservation_identity::*;
pub(crate) use snapshot_decoding::*;
pub(crate) use store_create_exclusive::*;
pub(crate) use transition_validation::*;
