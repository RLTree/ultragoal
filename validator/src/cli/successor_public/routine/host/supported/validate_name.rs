use super::*;

pub(crate) fn validate_name(name: &str) -> Result<(), HostFailure> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.as_bytes().contains(&b'/')
        || name.as_bytes().contains(&0)
    {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

pub(crate) fn identity(metadata: &fs::Metadata) -> Identity {
    Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
        owner: metadata.uid(),
        mode: metadata.mode(),
        links: metadata.nlink(),
    }
}

pub(crate) fn stat_identity(metadata: &libc::stat) -> Identity {
    Identity {
        device: metadata.st_dev as u64,
        inode: metadata.st_ino,
        owner: metadata.st_uid,
        mode: metadata.st_mode as u32,
        links: metadata.st_nlink as u64,
    }
}

pub(crate) fn same_anchored_directory(left: Identity, right: Identity) -> bool {
    left.device == right.device
        && left.inode == right.inode
        && left.owner == right.owner
        && left.mode == right.mode
}
