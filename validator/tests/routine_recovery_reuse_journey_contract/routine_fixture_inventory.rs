use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::MetadataExt;

use super::routine_fixture_workspace::{RoutineFixtureOwner, fixture_parent};

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct RoutineFixtureInventory {
    labels: Vec<String>,
    process_id: u32,
    owner_token: Option<String>,
    entries: BTreeMap<String, (u64, u64)>,
}

impl RoutineFixtureInventory {
    pub(crate) fn capture(labels: &[&str]) -> Self {
        let labels: Vec<String> = labels.iter().map(|value| (*value).to_owned()).collect();
        let process_id = std::process::id();
        let entries = inventory_entries(&labels, process_id, None);
        Self {
            labels,
            process_id,
            owner_token: None,
            entries,
        }
    }

    pub(crate) fn capture_owner(owner: &RoutineFixtureOwner) -> Self {
        let labels = Vec::new();
        let process_id = std::process::id();
        let owner_token = Some(owner.token().to_owned());
        let entries = inventory_entries(&labels, process_id, owner_token.as_deref());
        Self {
            labels,
            process_id,
            owner_token,
            entries,
        }
    }

    pub(crate) fn assert_unchanged(self) {
        let after = inventory_entries(&self.labels, self.process_id, self.owner_token.as_deref());
        assert_eq!(after, self.entries, "routine fixture inventory changed");
    }
}

fn inventory_entries(
    labels: &[String],
    process_id: u32,
    owner_token: Option<&str>,
) -> BTreeMap<String, (u64, u64)> {
    let entries = match fs::read_dir(fixture_parent()) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return BTreeMap::new(),
        Err(error) => panic!("routine fixture inventory failed: {error}"),
    };
    entries
        .map(|entry| entry.unwrap())
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let owned = match owner_token {
                Some(token) => name.contains(&format!("-{process_id}-{token}-")),
                None => labels
                    .iter()
                    .any(|label| name.starts_with(&format!("hul-routine-{label}-{process_id}-"))),
            };
            owned.then(|| {
                let metadata = fs::symlink_metadata(entry.path()).unwrap();
                (name, (metadata.dev(), metadata.ino()))
            })
        })
        .collect()
}
