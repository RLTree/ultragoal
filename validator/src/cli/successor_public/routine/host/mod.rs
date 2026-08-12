use std::fs::File;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

pub(crate) struct HostCustodyIssuance {
    authority_root: PathBuf,
    launch: HostLaunchIssuance,
    _seal: HostCustodySeal,
}

pub(crate) struct HostLaunchIssuance {
    path: PathBuf,
    file: File,
    device: u64,
    inode: u64,
    owner: u32,
    mode: u32,
    links: u64,
}

struct HostCustodySeal;

impl HostCustodyIssuance {
    fn new(authority_root: PathBuf, launch: HostLaunchIssuance) -> Self {
        Self {
            authority_root,
            launch,
            _seal: HostCustodySeal,
        }
    }

    fn from_host_state(state: &supported::HostState) -> Result<Self, HostFailure> {
        Ok(Self::new(
            state.authority.path.clone(),
            HostLaunchIssuance {
                path: state.launch.path.clone(),
                file: state
                    .launch
                    .file
                    .try_clone()
                    .map_err(|_| HostFailure::Invalid)?,
                device: state.launch.identity.device,
                inode: state.launch.identity.inode,
                owner: state.launch.identity.owner,
                mode: state.launch.identity.mode,
                links: state.launch.identity.links,
            },
        ))
    }

    #[cfg(test)]
    pub(crate) fn for_test(authority_root: &Path) -> Self {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
        let launch = authority_root
            .parent()
            .expect("test authority has parent")
            .join(supported::LAUNCH_DIRECTORY);
        if !launch.exists() {
            let mut builder = std::fs::DirBuilder::new();
            builder.mode(0o700);
            builder.create(&launch).expect("create test launch root");
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&launch)
            .expect("open test launch root");
        let metadata = file.metadata().expect("test launch metadata");
        Self::new(
            authority_root.to_path_buf(),
            HostLaunchIssuance {
                path: launch,
                file,
                device: metadata.dev(),
                inode: metadata.ino(),
                owner: metadata.uid(),
                mode: metadata.permissions().mode(),
                links: metadata.nlink(),
            },
        )
    }

    pub(crate) fn into_parts(self) -> (PathBuf, HostLaunchIssuance) {
        (self.authority_root, self.launch)
    }
}

impl HostLaunchIssuance {
    pub(crate) fn into_parts(self) -> (PathBuf, File, u64, u64, u32, u32, u64) {
        (
            self.path,
            self.file,
            self.device,
            self.inode,
            self.owner,
            self.mode,
            self.links,
        )
    }
}

#[cfg(target_vendor = "apple")]
#[path = "supported/mod.rs"]
mod supported;

pub(crate) use supported::ContinuationCheckpoint;

#[path = "host_failure.rs"]
mod host_failure;

#[path = "checkpoint_request.rs"]
mod checkpoint_request;

pub(crate) use checkpoint_request::*;
pub(crate) use host_failure::*;
