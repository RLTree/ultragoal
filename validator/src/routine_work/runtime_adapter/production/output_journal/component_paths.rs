use super::*;

pub(super) fn names_for(relative: &str) -> (&str, &str) {
    relative
        .rsplit_once('/')
        .map_or(("", relative), |(parent, child)| (parent, child))
}

pub(super) fn allowed_children(
    journal: &OutputProvisionJournal,
) -> BTreeMap<String, BTreeSet<String>> {
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
