#[derive(Clone, Debug)]
pub(crate) struct Store {
    root: PathBuf,
    canonical_root: PathBuf,
    directory: Arc<File>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}
