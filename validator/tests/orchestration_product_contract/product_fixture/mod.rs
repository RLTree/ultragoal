use crate::orchestration::*;
use crate::orchestration_product::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

include!("product_fixture/next_root.rs");

include!("product_fixture/reconcile_permit.rs");
