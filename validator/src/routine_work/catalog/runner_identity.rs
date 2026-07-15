use super::*;

fn same_ancestor_object(left: &DirectoryIdentity, right: &DirectoryIdentity) -> bool {
    left.relative_directory == right.relative_directory
        && left.device == right.device
        && left.inode == right.inode
        && left.unix_mode == right.unix_mode
        && left.owner_user_id == right.owner_user_id
        && left.owner_group_id == right.owner_group_id
}

pub(crate) fn same_runner_ancestors(
    left: &[DirectoryIdentity],
    right: &[DirectoryIdentity],
) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| same_ancestor_object(left, right))
}

pub(crate) fn same_runner_file(left: &FileIdentity, right: &FileIdentity) -> bool {
    left.device == right.device
        && left.inode == right.inode
        && left.unix_mode == right.unix_mode
        && left.owner_user_id == right.owner_user_id
        && left.owner_group_id == right.owner_group_id
        && left.link_count == right.link_count
        && left.byte_length == right.byte_length
        && left.modified_seconds == right.modified_seconds
        && left.modified_nanos == right.modified_nanos
        && left.changed_seconds == right.changed_seconds
        && left.changed_nanos == right.changed_nanos
        && left.sha256 == right.sha256
        && same_runner_ancestors(&left.ancestors, &right.ancestors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ancestor(modified_seconds: i64, inode: u64) -> DirectoryIdentity {
        DirectoryIdentity {
            relative_directory: "/fixture-parent".to_owned(),
            device: 1,
            inode,
            unix_mode: 0o40755,
            owner_user_id: 501,
            owner_group_id: 20,
            modified_seconds,
            modified_nanos: 0,
            changed_seconds: modified_seconds,
            changed_nanos: 0,
        }
    }

    #[test]
    fn sibling_directory_activity_is_not_runner_object_substitution() {
        assert!(same_runner_ancestors(
            &[ancestor(10, 7)],
            &[ancestor(11, 7)]
        ));
        assert!(!same_runner_ancestors(
            &[ancestor(10, 7)],
            &[ancestor(10, 8)]
        ));
    }
}
