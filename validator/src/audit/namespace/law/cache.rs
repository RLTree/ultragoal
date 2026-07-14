use crate::audit::source_governance::GovernedInventory;
use std::path::Path;

#[derive(Default)]
pub(crate) struct ValueCache {
    actual_files: Option<Vec<String>>,
    governed_inventory: Option<GovernedInventory>,
    governed_failures: Option<Vec<String>>,
    repo_source_paths: Option<Vec<String>>,
}

impl ValueCache {
    pub(super) fn actual_files(&mut self, root: &Path) -> Vec<String> {
        self.actual_files
            .get_or_insert_with(|| {
                crate::package::inventory::closure::actual_files(root).unwrap_or_default()
            })
            .clone()
    }

    pub(super) fn repo_source_paths(&mut self, root: &Path) -> Vec<String> {
        if let Some(paths) = &self.repo_source_paths {
            return paths.clone();
        }
        let (inventory, _) = self.governed_snapshot(root);
        let paths = inventory
            .sources
            .iter()
            .map(|source| source.relative.clone())
            .collect::<Vec<_>>();
        let paths =
            crate::audit::namespace::source::topology::repo_source_paths_from_actual_files(&paths);
        self.repo_source_paths = Some(paths.clone());
        paths
    }

    pub(super) fn governed_snapshot(&mut self, root: &Path) -> (GovernedInventory, Vec<String>) {
        if self.governed_inventory.is_none() {
            let audit = crate::audit::source_governance::audit(root);
            self.governed_inventory = Some(audit.inventory);
            self.governed_failures = Some(audit.failures);
        }
        (
            self.governed_inventory
                .as_ref()
                .expect("governed inventory initialized")
                .clone(),
            self.governed_failures
                .as_ref()
                .expect("governed failures initialized")
                .clone(),
        )
    }
}
