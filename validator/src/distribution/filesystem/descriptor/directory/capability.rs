pub(crate) struct Directory {
    anchor: Arc<Anchor>,
    identity: DirectoryIdentity,
    root_device: u64,
    anchor_relative: String,
    relative: String,
}

/// The only retained filesystem capability for a distribution transaction.
///
/// Descendants are identities plus paths below this descriptor, not retained
/// mutation descriptors. A descendant can therefore never become authority
/// merely because it was opened while it happened to have a confined name.
struct Anchor {
    file: File,
    identity: DirectoryIdentity,
    authority: StableDirectoryAuthority,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StableDirectoryAuthority {
    identity: DirectoryIdentity,
    mode: u32,
    uid: u32,
    gid: u32,
}

impl StableDirectoryAuthority {
    fn capture(metadata: &std::fs::Metadata) -> Result<Self, DistributionError> {
        if !metadata.is_dir() {
            return Err(error(DistributionErrorId::UnsafeObject));
        }
        Ok(Self {
            identity: directory_identity(metadata),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
        })
    }

    fn confined_root(metadata: &std::fs::Metadata) -> Result<Self, DistributionError> {
        let authority = Self::capture(metadata)?;
        if authority.uid != unsafe { libc::geteuid() } || authority.mode & 0o022 != 0 {
            return Err(error(DistributionErrorId::UnsafeObject));
        }
        Ok(authority)
    }
}

impl std::fmt::Debug for Directory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Directory")
            .field("identity", &self.identity)
            .field("root_device", &self.root_device)
            .field("anchor_relative", &self.anchor_relative)
            .field("relative", &self.relative)
            .finish()
    }
}
