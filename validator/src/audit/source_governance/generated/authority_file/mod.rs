mod identity;

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod descriptor_path;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod unix_descriptor;

pub(in crate::audit::source_governance) use identity::AuthorityFileBaseline;
use identity::BaselineChange;
use std::path::Path;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use unix_descriptor as platform;

pub(in crate::audit::source_governance) struct AuthorityRoot {
    handle: platform::RootHandle,
}

pub(in crate::audit::source_governance) struct AuthorityFileCapture {
    pub(in crate::audit::source_governance) bytes: Vec<u8>,
    pub(in crate::audit::source_governance) baseline: AuthorityFileBaseline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuthorityFileReadErrorId {
    PathInvalid,
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    PlatformUnsupported,
    RootUnavailable,
    RootRejected,
    AncestorRejected,
    LeafRejected,
    LeafNotRegular,
    LeafLinked,
    ReadFailed,
    ChangedDuringRead,
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::audit::source_governance) struct AuthorityFileReadError {
    id: AuthorityFileReadErrorId,
}

impl AuthorityFileReadError {
    pub(in crate::audit::source_governance) fn stable_text(&self) -> &'static str {
        read_error_text(self.id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuthorityFileRevalidationErrorId {
    Capture(AuthorityFileReadErrorId),
    RootIdentityChanged,
    AncestorIdentityChanged,
    LeafIdentityChanged,
    LeafContentChanged,
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::audit::source_governance) struct AuthorityFileRevalidationError {
    id: AuthorityFileRevalidationErrorId,
}

impl AuthorityFileRevalidationError {
    pub(in crate::audit::source_governance) fn stable_text(&self) -> &'static str {
        match self.id {
            AuthorityFileRevalidationErrorId::Capture(id) => read_error_text(id),
            AuthorityFileRevalidationErrorId::RootIdentityChanged => {
                "authority_file_root_identity_changed"
            }
            AuthorityFileRevalidationErrorId::AncestorIdentityChanged => {
                "authority_file_ancestor_identity_changed"
            }
            AuthorityFileRevalidationErrorId::LeafIdentityChanged => {
                "authority_file_leaf_identity_changed"
            }
            AuthorityFileRevalidationErrorId::LeafContentChanged => {
                "authority_file_content_changed"
            }
        }
    }

    pub(in crate::audit::source_governance) fn content_changed(&self) -> bool {
        self.id == AuthorityFileRevalidationErrorId::LeafContentChanged
    }
}

impl AuthorityRoot {
    pub(in crate::audit::source_governance) fn open(
        path: &Path,
    ) -> Result<Self, AuthorityFileReadError> {
        Ok(Self {
            handle: platform::open_root(path)?,
        })
    }

    pub(in crate::audit::source_governance) fn capture(
        &self,
        relative: &Path,
    ) -> Result<AuthorityFileCapture, AuthorityFileReadError> {
        platform::capture(&self.handle, relative)
    }
}

pub(in crate::audit::source_governance) fn revalidate(
    root: &Path,
    expected_bytes: &[u8],
    baseline: &AuthorityFileBaseline,
) -> Result<(), AuthorityFileRevalidationError> {
    let authority = AuthorityRoot::open(root).map_err(revalidation_capture_error)?;
    let current = authority
        .capture(baseline.relative())
        .map_err(revalidation_capture_error)?;
    let change = baseline.change(&current.baseline);
    match change {
        Some(BaselineChange::Root) => Err(revalidation_error(
            AuthorityFileRevalidationErrorId::RootIdentityChanged,
        )),
        Some(BaselineChange::Ancestor) => Err(revalidation_error(
            AuthorityFileRevalidationErrorId::AncestorIdentityChanged,
        )),
        Some(BaselineChange::Leaf) => Err(revalidation_error(
            AuthorityFileRevalidationErrorId::LeafIdentityChanged,
        )),
        Some(BaselineChange::Content) => Err(revalidation_error(
            AuthorityFileRevalidationErrorId::LeafContentChanged,
        )),
        None if current.bytes == expected_bytes => Ok(()),
        None => Err(revalidation_error(
            AuthorityFileRevalidationErrorId::LeafContentChanged,
        )),
    }
}

fn error(id: AuthorityFileReadErrorId) -> AuthorityFileReadError {
    AuthorityFileReadError { id }
}

fn revalidation_capture_error(error: AuthorityFileReadError) -> AuthorityFileRevalidationError {
    revalidation_error(AuthorityFileRevalidationErrorId::Capture(error.id))
}

fn revalidation_error(id: AuthorityFileRevalidationErrorId) -> AuthorityFileRevalidationError {
    AuthorityFileRevalidationError { id }
}

fn read_error_text(id: AuthorityFileReadErrorId) -> &'static str {
    match id {
        AuthorityFileReadErrorId::PathInvalid => "authority_file_path_invalid",
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        AuthorityFileReadErrorId::PlatformUnsupported => {
            "authority_file_descriptor_confinement_unsupported"
        }
        AuthorityFileReadErrorId::RootUnavailable => "authority_file_root_unavailable",
        AuthorityFileReadErrorId::RootRejected => "authority_file_root_rejected",
        AuthorityFileReadErrorId::AncestorRejected => "authority_file_ancestor_rejected",
        AuthorityFileReadErrorId::LeafRejected => "authority_file_leaf_rejected",
        AuthorityFileReadErrorId::LeafNotRegular => "authority_file_regular_leaf_required",
        AuthorityFileReadErrorId::LeafLinked => "authority_file_linked_leaf_rejected",
        AuthorityFileReadErrorId::ReadFailed => "authority_file_read_failed",
        AuthorityFileReadErrorId::ChangedDuringRead => "authority_file_changed_during_read",
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod platform {
    use super::{AuthorityFileCapture, AuthorityFileReadError, AuthorityFileReadErrorId, error};
    use std::path::Path;

    pub(super) struct RootHandle;

    pub(super) fn open_root(_path: &Path) -> Result<RootHandle, AuthorityFileReadError> {
        Err(error(AuthorityFileReadErrorId::PlatformUnsupported))
    }

    pub(super) fn capture(
        _root: &RootHandle,
        _relative: &Path,
    ) -> Result<AuthorityFileCapture, AuthorityFileReadError> {
        Err(error(AuthorityFileReadErrorId::PlatformUnsupported))
    }
}
