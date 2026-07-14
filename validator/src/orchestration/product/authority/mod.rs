use super::ProductError;
use crate::orchestration::{Actor, Binding, EffectResolution};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Debug, Formatter};

include!("authority_schema.rs");

include!("root/secret_binding.rs");

include!("root/verification.rs");

include!("issue_action_permit_for_test.rs");
