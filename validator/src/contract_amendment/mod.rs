mod contracts;
mod validation;

#[cfg(test)]
mod tests;

pub(crate) use contracts::{CurrentAmendmentBinding, ValidatedCurrentAmendment};
pub(crate) use validation::validate_current;
