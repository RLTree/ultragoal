use super::*;

pub(crate) static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

pub(crate) struct ParentAnchor {
    pub(crate) directory: File,
    pub(crate) canonical: PathBuf,
    pub(crate) attachments: Vec<(File, String, ObjectIdentity)>,
}

pub(crate) struct PrivateTransaction {
    pub(crate) name: String,
    pub(crate) directory: File,
    pub(crate) canonical: PathBuf,
    pub(crate) identity: ObjectIdentity,
}

#[derive(Clone)]
pub(crate) struct CreatedDirectory {
    pub(crate) path: String,
    pub(crate) identity: ObjectIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ObjectIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
}

pub(crate) struct ObservedLeaf {
    pub(crate) file: File,
    pub(crate) stat: PathStat,
    pub(crate) bytes: Vec<u8>,
    pub(crate) mode: u32,
}

/// Concrete local effect adapter. Construction binds one directory
/// descriptor and never follows links or consults host configuration.
pub(crate) struct LocalEffects {
    pub(crate) path: PathBuf,
    pub(crate) canonical: PathBuf,
    pub(crate) root: File,
    pub(crate) root_identity: ObjectIdentity,
    pub(crate) binding: String,
    pub(crate) reader: LocalRepository,
    pub(crate) mutation_lease: bool,
    pub(crate) unix_modes: BTreeMap<String, u32>,
    pub(crate) rollback_modes: BTreeMap<String, (String, u32)>,
    pub(crate) created_directories: BTreeMap<String, ObjectIdentity>,
    #[cfg(test)]
    pub(crate) fail_calls: BTreeSet<usize>,
    #[cfg(test)]
    pub(crate) compare_calls: usize,
    #[cfg(test)]
    pub(crate) before_linearize: Option<(Arc<Barrier>, Arc<Barrier>)>,
    #[cfg(test)]
    pub(crate) after_replace_swap: Option<(Arc<Barrier>, Arc<Barrier>)>,
    #[cfg(test)]
    pub(crate) before_quarantine_move: Option<(Arc<Barrier>, Arc<Barrier>)>,
    #[cfg(test)]
    pub(crate) after_directory_publish: Option<(Arc<Barrier>, Arc<Barrier>)>,
    #[cfg(test)]
    pub(crate) before_directory_quarantine_move: Option<(Arc<Barrier>, Arc<Barrier>)>,
}
