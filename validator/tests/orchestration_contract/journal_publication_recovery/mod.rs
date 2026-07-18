use super::durable_journal_fixture::*;
use super::orchestration_fixture::*;
use crate::orchestration::*;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Barrier};

include!("child_mode.rs");

include!("prepared_recovery_cannot_follow_a_regular_root_substitution.rs");
