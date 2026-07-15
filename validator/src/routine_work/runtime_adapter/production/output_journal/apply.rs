use super::creation::{ProvisionEvent, provision};
use super::directory_entries::names;
use super::observation::{open_at, open_root, stat_at, validate_directory};
use super::*;

pub(super) fn apply(
    ledger: &FileAuthorityLedger,
    token: &ReservationToken,
    root: &Path,
) -> Result<(), RoutineError> {
    apply_inner(ledger, token, root, &mut |_, _| Ok(()))
}

#[cfg(test)]
pub(super) enum ApplyEvent<'a> {
    StageCreated(&'a str),
    StageRecorded(&'a str),
    Published(&'a str),
    FinalRecorded(&'a str),
}

#[cfg(test)]
pub(super) fn apply_observed(
    ledger: &FileAuthorityLedger,
    token: &ReservationToken,
    root: &Path,
    observer: &mut dyn FnMut(ApplyEvent<'_>) -> Result<(), RoutineError>,
) -> Result<(), RoutineError> {
    apply_inner(ledger, token, root, &mut |path, event| {
        observer(match event {
            ProvisionEvent::StageCreated => ApplyEvent::StageCreated(path),
            ProvisionEvent::StageRecorded => ApplyEvent::StageRecorded(path),
            ProvisionEvent::Published => ApplyEvent::Published(path),
            ProvisionEvent::FinalRecorded => ApplyEvent::FinalRecorded(path),
        })
    })
}

fn apply_inner(
    ledger: &FileAuthorityLedger,
    token: &ReservationToken,
    root: &Path,
    observer: &mut dyn FnMut(&str, ProvisionEvent) -> Result<(), RoutineError>,
) -> Result<(), RoutineError> {
    let journal = &token.output_journal;
    let root = open_root(root)?;
    if identity(
        &root
            .metadata()
            .map_err(|_| error("routine-production-output-root-stat-failed"))?,
    ) != journal.root
    {
        return Err(error("routine-production-output-root-changed"));
    }
    let allowed = allowed_children(journal);
    let mut opened = BTreeMap::new();
    opened.insert(String::new(), root);
    for component in &journal.components {
        let (parent_name, child_name) = component
            .relative_path
            .rsplit_once('/')
            .map_or(("", component.relative_path.as_str()), |(parent, child)| {
                (parent, child)
            });
        let parent = opened
            .get(parent_name)
            .ok_or_else(|| error("routine-production-output-parent-unbound"))?;
        let observed = stat_at(parent, child_name)?;
        let expected = match (component.preexisting, observed) {
            (Some(expected), Some(current)) if expected == current => current,
            (Some(_), _) => return Err(error("routine-production-output-prestate-changed")),
            (None, _) => provision(
                ledger,
                token,
                component,
                parent,
                child_name,
                journal.root.device,
                observer,
            )?,
        };
        validate_directory(expected, journal.root.device)?;
        let child = open_at(parent, child_name)?;
        if identity(
            &child
                .metadata()
                .map_err(|_| error("routine-production-output-stat-failed"))?,
        ) != expected
            || stat_at(parent, child_name)? != Some(expected)
        {
            return Err(error("routine-production-output-open-raced"));
        }
        if component.preexisting.is_none() {
            let expected_names = allowed
                .get(&component.relative_path)
                .cloned()
                .unwrap_or_default();
            if !names(&child)?.is_subset(&expected_names) {
                return Err(error("routine-production-output-owned-content-ambiguous"));
            }
        }
        opened.insert(component.relative_path.clone(), child);
    }
    Ok(())
}

fn allowed_children(journal: &OutputProvisionJournal) -> BTreeMap<String, BTreeSet<String>> {
    let mut allowed = BTreeMap::<String, BTreeSet<String>>::new();
    for component in &journal.components {
        if let Some((parent, child)) = component.relative_path.rsplit_once('/') {
            let children = allowed.entry(parent.to_owned()).or_default();
            children.insert(child.to_owned());
            if component.preexisting.is_none()
                && let Some(nonce) = component.creation_nonce.as_deref()
            {
                children.insert(format!(".routine-output-{nonce}"));
            }
        }
    }
    allowed
}
