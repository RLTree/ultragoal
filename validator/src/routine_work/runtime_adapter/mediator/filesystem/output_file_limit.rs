use super::*;

pub(crate) const MAX_OUTPUT_FILES: usize = 100_000;
pub(crate) const MAX_READ_SOURCE_BYTES: u64 = 64 * 1024 * 1024;

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ObjectIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) mode: u32,
    pub(crate) owner_user_id: u32,
    pub(crate) owner_group_id: u32,
    pub(crate) links: u64,
    pub(crate) length: u64,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanos: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
}

#[cfg(unix)]
impl ObjectIdentity {
    pub(crate) fn from(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            owner_user_id: metadata.uid(),
            owner_group_id: metadata.gid(),
            links: metadata.nlink(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
        }
    }
}

#[cfg(unix)]
pub(crate) struct ReadAncestorAnchor {
    pub(crate) file: File,
    pub(crate) identity: ObjectIdentity,
}

#[cfg(unix)]
pub(crate) struct ReadSourceAnchor {
    pub(crate) path: PathBuf,
    pub(crate) file: File,
    pub(crate) identity: ObjectIdentity,
    pub(crate) ancestors: Vec<ReadAncestorAnchor>,
    pub(crate) record: RoutineReadSource,
}

pub(crate) struct ReadConfinement {
    #[cfg(unix)]
    pub(crate) sources: Vec<ReadSourceAnchor>,
}
