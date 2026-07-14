use super::*;

pub(crate) struct Workspace {
    pub(crate) path: PathBuf,
    pub(crate) canonical: PathBuf,
    pub(crate) root: File,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) binding: String,
}
