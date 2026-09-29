mod contracts;
mod validation;

#[cfg(test)]
mod tests;

pub(crate) use contracts::{CurrentAmendmentBinding, ExpectedArtifactBinding};
pub(crate) use validation::validate_current;
