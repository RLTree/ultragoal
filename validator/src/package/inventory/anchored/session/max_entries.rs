const MAX_ENTRIES: u64 = 100_000;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

pub(crate) struct Session {
    root: PathBuf,
    root_file: File,
    root_snapshot: Snapshot,
    strict_root_snapshot: Option<Snapshot>,
    observed: BTreeMap<PathBuf, Snapshot>,
    entries: u64,
    bytes: u64,
}
