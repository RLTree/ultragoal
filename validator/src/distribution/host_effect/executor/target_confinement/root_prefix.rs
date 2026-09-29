const ROOT_PREFIX: &str = "hul-supported-host-effect-";
const ISOLATED_PACKAGE_ROOT_PREFIX: &str = "hul-distribution-host-effect-";
const MAX_PUBLICATION_BYTES: usize = 1024 * 1024;
const PUBLICATION_MODE: u32 = 0o400;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StableDirectoryIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
}

impl StableDirectoryIdentity {
    fn capture(metadata: &Metadata) -> Result<Self, HostEffectExecutorFailure> {
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.mode() & 0o777 != 0o700
            || metadata.uid() != unsafe { libc::geteuid() }
        {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::InvalidTargetRoot,
            ));
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
        })
    }
}

struct TargetRootAnchor {
    canonical_path: PathBuf,
    parent: File,
    directory: File,
    name: CString,
    identity: StableDirectoryIdentity,
    scope: AcceptedHostScope,
    target_generation: u64,
    expected_target: ObservedTargetIdentity,
}

#[derive(Clone)]
pub(crate) struct ConfinedHostEffectTarget {
    anchor: Arc<TargetRootAnchor>,
}

impl std::fmt::Debug for ConfinedHostEffectTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConfinedHostEffectTarget")
            .field("canonical_path", &self.anchor.canonical_path)
            .field("target_generation", &self.anchor.target_generation)
            .finish_non_exhaustive()
    }
}

impl ConfinedHostEffectTarget {
    pub(in crate::distribution::host_effect) fn cwd_fd(&self) -> std::os::fd::RawFd {
        self.anchor.directory.as_raw_fd()
    }
}
