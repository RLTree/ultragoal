use crate::durable_journal_fixture::*;
use crate::orchestration::*;
use crate::orchestration_fixture::*;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Barrier};

include!("child_mode.rs");

include!("prepared_recovery_cannot_follow_a_regular_root_substitution.rs");
