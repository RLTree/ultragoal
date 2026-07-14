mod model;
mod policy;
mod registry_parser;

use super::GovernedSource;
use std::path::Path;

pub(super) fn failures(root: &Path, sources: &[GovernedSource]) -> Vec<String> {
    policy::failures(root, sources)
}
