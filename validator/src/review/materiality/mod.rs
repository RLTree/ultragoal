use crate::review::round::{
    ReviewFailure,
    anchor::{
        policy::{self, AnchorSource},
        values::AnchorValues,
    },
    config::{
        self, MATERIALITY_ADVISORY as ADVISORY, MATERIALITY_BLOCKED as BLOCKED,
        MATERIALITY_DELTA as DELTA, MATERIALITY_FULL as FULL, MaterialityAuthority,
        REVIEW_ROLES as ROLES,
    },
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

mod registry;
use registry::registry_contents_valid;

include!("review_round_errors.rs");

include!("full_errors.rs");
