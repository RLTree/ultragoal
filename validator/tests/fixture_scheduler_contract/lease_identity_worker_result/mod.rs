use crate::orchestration::*;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

include!("result/envelope.rs");

include!("result/artifact_verification.rs");
