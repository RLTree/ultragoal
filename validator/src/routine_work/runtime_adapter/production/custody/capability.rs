use super::*;
use crate::cli::successor_public::routine::HostCustodyIssuance;
use crate::routine_work::runtime_adapter::mediator::ObjectIdentity;
use std::fs::File;

/// Opaque authority to enter the private routine custody transaction.
///
/// The host adapter issues this non-clone capability after opening and
/// validating its anchored state. Only the private custody owner can recover
/// the authority path.
pub(crate) struct RoutineCustodyCapability {
    authority_root: PathBuf,
    launch: LaunchDirectoryCapability,
    _seal: CapabilitySeal,
}

struct LaunchDirectoryCapability {
    path: PathBuf,
    file: File,
    device: u64,
    inode: u64,
    owner: u32,
    mode: u32,
    links: u64,
}

struct CapabilitySeal;

impl RoutineCustodyCapability {
    pub(crate) fn issue_from_host(issuance: HostCustodyIssuance) -> Self {
        let (authority_root, launch) = issuance.into_parts();
        let (path, file, device, inode, owner, mode, links) = launch.into_parts();
        Self {
            authority_root,
            launch: LaunchDirectoryCapability {
                path,
                file,
                device,
                inode,
                owner,
                mode,
                links,
            },
            _seal: CapabilitySeal,
        }
    }

    #[cfg(test)]
    pub(super) fn issue_for_test(authority_root: &Path) -> Self {
        Self::issue_from_host(HostCustodyIssuance::for_test(authority_root))
    }

    pub(in crate::routine_work::runtime_adapter::production) fn authority_root(&self) -> &Path {
        &self.authority_root
    }

    pub(in crate::routine_work::runtime_adapter::production) fn launch_directory(
        &self,
    ) -> Result<(PathBuf, File, ObjectIdentity), RoutineError> {
        let file = self
            .launch
            .file
            .try_clone()
            .map_err(|_| error("routine-production-launch-directory-duplicate-failed"))?;
        let metadata = file
            .metadata()
            .map_err(|_| error("routine-production-launch-directory-stat-failed"))?;
        let identity = ObjectIdentity::from(&metadata);
        let current = std::fs::symlink_metadata(&self.launch.path)
            .map_err(|_| error("routine-production-launch-directory-replaced"))?;
        if current.file_type().is_symlink()
            || !current.is_dir()
            || identity.device != self.launch.device
            || identity.inode != self.launch.inode
            || identity.owner_user_id != self.launch.owner
            || identity.mode != self.launch.mode
            || identity.links != self.launch.links
            || ObjectIdentity::from(&current) != identity
        {
            return Err(error("routine-production-launch-directory-replaced"));
        }
        Ok((self.launch.path.clone(), file, identity))
    }
}
