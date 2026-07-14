use super::*;
use crate::distribution::{PackageIdentity, SourceIdentity};

#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};

include!("execution_authority.rs");

include!("authorized_effect_enforces_exact_live_executable_identity.rs");
