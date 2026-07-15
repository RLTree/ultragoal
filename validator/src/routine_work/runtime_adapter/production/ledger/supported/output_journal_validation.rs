use super::*;

pub(crate) fn validate_output_journal(
    journal: &OutputProvisionJournal,
) -> Result<(), RoutineError> {
    let mut scopes = journal.scopes.clone();
    scopes.sort();
    scopes.dedup();
    let expected_components = component_paths(&scopes);
    let actual_components = journal
        .components
        .iter()
        .map(|component| component.relative_path.clone())
        .collect::<Vec<_>>();
    if !valid_identity(&journal.root)
        || journal.scopes.len() > 128
        || journal.components.len() > 384
        || scopes != journal.scopes
        || journal.scopes.iter().any(|scope| !valid_repo_path(scope))
        || actual_components != expected_components
        || journal.components.iter().any(|component| {
            component
                .preexisting
                .as_ref()
                .is_some_and(|value| !valid_identity(value))
                || component
                    .provisioned
                    .as_ref()
                    .is_some_and(|value| !valid_identity(value))
                || component.preexisting.is_some() && component.provisioned.is_some()
        })
        || impossible_descendant_state(journal)
    {
        return Err(error("routine-production-output-journal-invalid"));
    }
    Ok(())
}

fn component_paths(scopes: &[String]) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for scope in scopes {
        let mut current = String::new();
        for component in scope.split('/') {
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(component);
            paths.insert(current.clone());
        }
    }
    let mut paths = paths.into_iter().collect::<Vec<_>>();
    paths.sort_by(|left, right| {
        left.matches('/')
            .count()
            .cmp(&right.matches('/').count())
            .then_with(|| left.cmp(right))
    });
    paths
}

fn impossible_descendant_state(journal: &OutputProvisionJournal) -> bool {
    let states = journal
        .components
        .iter()
        .map(|component| (component.relative_path.as_str(), component))
        .collect::<BTreeMap<_, _>>();
    journal.components.iter().any(|component| {
        let Some((parent, _)) = component.relative_path.rsplit_once('/') else {
            return false;
        };
        states.get(parent).is_some_and(|parent| {
            parent.preexisting.is_none()
                && parent.provisioned.is_none()
                && (component.preexisting.is_some() || component.provisioned.is_some())
        })
    })
}

fn valid_identity(identity: &OutputDirectoryIdentity) -> bool {
    identity.device != 0
        && identity.inode != 0
        && identity.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFDIR)
}

fn valid_repo_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.split('/').all(|component| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && component
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
}
