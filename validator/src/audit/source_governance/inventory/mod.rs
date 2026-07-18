mod scan;

use super::generated;
use super::generated::authority_file::{
    AuthorityFileBaseline, AuthorityFileRevalidationError, revalidate,
};
use super::scope::SourceClass;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GovernedSource {
    pub(crate) relative: String,
    pub(crate) class: SourceClass,
    pub(crate) bytes: Vec<u8>,
    authority: AuthorityFileBaseline,
}

#[derive(Clone, Debug)]
pub(crate) struct GovernedInventory {
    pub(crate) sources: Vec<GovernedSource>,
    pub(crate) generated_projections: std::collections::BTreeSet<String>,
}

pub(crate) struct GovernedAudit {
    pub(crate) inventory: GovernedInventory,
    pub(crate) failures: Vec<String>,
}

#[cfg(test)]
pub(crate) fn capture(root: &Path) -> Result<GovernedInventory, Vec<String>> {
    let result = audit(root);
    if result.failures.is_empty() {
        Ok(result.inventory)
    } else {
        Err(result.failures)
    }
}

pub(crate) fn audit(root: &Path) -> GovernedAudit {
    let (first, mut failures) = scan::once(root);
    let (mut second, second_failures) = scan::once(root);
    failures.extend(second_failures);
    if first.sources != second.sources {
        failures.push("governed_source_inventory_changed_during_capture".to_string());
    }
    let generated = generated::validate(root, &second.sources);
    failures.extend(generated.failures);
    second.generated_projections = generated.source_projections;
    failures.extend(super::live_root_document::failures(&second.sources));
    failures.extend(super::standards_integrity::failures(root, &second.sources));
    failures.extend(super::lint_allowance::failures(&second));
    let (final_inventory, final_failures) = scan::once(root);
    failures.extend(final_failures);
    if final_inventory.sources != second.sources {
        failures.push("governed_source_inventory_changed_during_validation".to_string());
    }
    failures.sort();
    failures.dedup();
    GovernedAudit {
        inventory: second,
        failures,
    }
}

impl GovernedSource {
    pub(in crate::audit::source_governance) fn revalidate(
        &self,
        root: &Path,
    ) -> Result<(), AuthorityFileRevalidationError> {
        revalidate(root, &self.bytes, &self.authority)
    }
}

#[cfg(test)]
pub(super) fn capture_once_for_test(root: &Path) -> Result<GovernedInventory, Vec<String>> {
    capture_once(root)
}

#[cfg(test)]
pub(super) fn capture_with_between_for_test(
    root: &Path,
    between: impl FnOnce(),
) -> Result<GovernedInventory, Vec<String>> {
    let (first, first_failures) = scan::once(root);
    if !first_failures.is_empty() {
        return Err(first_failures);
    }
    between();
    let (second, second_failures) = scan::once(root);
    if !second_failures.is_empty() {
        return Err(second_failures);
    }
    if first.sources != second.sources {
        Err(vec![
            "governed_source_inventory_changed_during_capture".to_string(),
        ])
    } else {
        Ok(second)
    }
}

#[cfg(test)]
fn capture_once(root: &Path) -> Result<GovernedInventory, Vec<String>> {
    let (inventory, failures) = scan::once(root);
    if failures.is_empty() {
        Ok(inventory)
    } else {
        Err(failures)
    }
}
