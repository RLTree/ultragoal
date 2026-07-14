use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU8, AtomicU64, Ordering as AtomicOrdering},
};

include!("sha256_prefix.rs");

include!("plan_authorization_seal_issue.rs");

include!("recovery_authorization_seal_issue.rs");

include!("validate_digest.rs");
