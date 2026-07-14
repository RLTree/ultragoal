use super::scope::path_is_within;
use super::{CanonicalPath, LeaseSpec, OrchestrationError, WorkPackage, WorkerResultV1};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, Metadata};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

include!("max_artifact_bytes.rs");

include!("verify_file.rs");
