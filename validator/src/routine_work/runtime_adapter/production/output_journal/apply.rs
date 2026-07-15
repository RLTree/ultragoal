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
    Created(&'a str),
    Recorded(&'a str),
}

#[cfg(test)]
pub(super) fn apply_observed(
    ledger: &FileAuthorityLedger,
    token: &ReservationToken,
    root: &Path,
    observer: &mut dyn FnMut(ApplyEvent<'_>) -> Result<(), RoutineError>,
) -> Result<(), RoutineError> {
    apply_inner(ledger, token, root, &mut |path, recorded| {
        observer(if recorded {
            ApplyEvent::Recorded(path)
        } else {
            ApplyEvent::Created(path)
        })
    })
}

fn apply_inner(
    ledger: &FileAuthorityLedger,
    token: &ReservationToken,
    root: &Path,
    observer: &mut dyn FnMut(&str, bool) -> Result<(), RoutineError>,
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
        let expected = match (component.preexisting, component.provisioned, observed) {
            (Some(expected), _, Some(current)) if expected == current => current,
            (Some(_), _, _) => return Err(error("routine-production-output-prestate-changed")),
            (None, Some(expected), Some(current)) if expected == current => current,
            (None, Some(_), _) => {
                return Err(error("routine-production-output-owned-state-changed"));
            }
            (None, None, Some(current)) if !token.reuse_only => {
                validate_adoptable(current, journal.root.device)?;
                current
            }
            (None, None, None) if !token.reuse_only => {
                mkdir_at(parent, child_name)?;
                let created = stat_at(parent, child_name)?
                    .ok_or_else(|| error("routine-production-output-create-unobserved"))?;
                observer(&component.relative_path, false)?;
                created
            }
            (None, None, _) => return Err(error("routine-production-reuse-output-missing")),
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
            validate_adoptable(expected, journal.root.device)?;
            let expected_names = allowed
                .get(&component.relative_path)
                .cloned()
                .unwrap_or_default();
            if !names(&child)?.is_subset(&expected_names) {
                return Err(error("routine-production-output-owned-content-ambiguous"));
            }
        }
        if !token.reuse_only && component.preexisting.is_none() {
            ledger.record_output_component(token, &component.relative_path, expected)?;
            observer(&component.relative_path, true)?;
        }
        opened.insert(component.relative_path.clone(), child);
    }
    Ok(())
}

fn allowed_children(journal: &OutputProvisionJournal) -> BTreeMap<String, BTreeSet<String>> {
    let mut allowed = BTreeMap::<String, BTreeSet<String>>::new();
    for component in &journal.components {
        if let Some((parent, child)) = component.relative_path.rsplit_once('/') {
            allowed
                .entry(parent.to_owned())
                .or_default()
                .insert(child.to_owned());
        }
    }
    allowed
}

fn validate_adoptable(
    identity: OutputDirectoryIdentity,
    root_device: u64,
) -> Result<(), RoutineError> {
    validate_directory(identity, root_device)?;
    if identity.owner != unsafe { libc::geteuid() } || identity.mode & 0o7777 != 0o700 {
        return Err(error("routine-production-output-unowned-state"));
    }
    Ok(())
}

fn mkdir_at(parent: &File, name: &str) -> Result<(), RoutineError> {
    let name = validate_name(name)?;
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
        return Err(error("routine-production-output-create-failed"));
    }
    Ok(())
}
