use crate::durable_journal_fixture::*;
use crate::orchestration::*;
use crate::orchestration_fixture::*;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Barrier};

include!("journal_publication_recovery/child_mode.rs");

include!(
    "journal_publication_recovery/prepared_recovery_cannot_follow_a_regular_root_substitution.rs"
);
