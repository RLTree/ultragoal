use super::fresh_process_child::{CHILD_ENV, CHILD_TEST, ChildInput, ChildOutput};
use super::state_fixture::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::process::{Child, Command};

include!("fresh_process/write_input.rs");

include!("fresh_process/concurrent_fresh_process_recovery_has_one_authoritative_outcome.rs");
