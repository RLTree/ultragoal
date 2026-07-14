use super::{HostContext, HostError, TrustedTime, digest_bytes};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::{Arc, Mutex};

use super::super::{
    ApplyAuthorizationAuthority, MigrationInputBinding, MigrationOperation, ProductMigrationError,
    ProductMigrationPlan,
};
use serde::Deserialize;

include!("authorization_seal_domain.rs");

include!("darwin/issuance.rs");

include!("darwin/principal.rs");
