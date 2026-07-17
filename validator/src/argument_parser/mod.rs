#[cfg(not(test))]
use crate::Args;
#[cfg(test)]
use crate::audit;
use crate::cli;
#[cfg(test)]
use crate::command::{Args, Command};
#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
mod authority;
#[cfg(test)]
mod help_request;
#[cfg(test)]
mod specialized;
#[cfg(test)]
mod tests;

#[path = "public_arguments.rs"]
mod public_arguments;

pub(crate) use public_arguments::*;
