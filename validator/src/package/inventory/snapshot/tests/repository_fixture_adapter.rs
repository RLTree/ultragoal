use std::path::Path;
use std::process::Command;

pub(super) enum RepositoryFixtureRequest<'a> {
    Initialize { root: &'a Path },
    Status { root: &'a Path },
}

pub(super) enum RepositoryFixtureResponse {
    Initialized,
    Status(Vec<u8>),
}

#[derive(Debug)]
pub(super) enum RepositoryFixtureError {
    Spawn,
    Rejected,
    UnexpectedResponse,
}

fn execute(
    request: RepositoryFixtureRequest<'_>,
) -> Result<RepositoryFixtureResponse, RepositoryFixtureError> {
    let (root, arguments): (&Path, &[&str]) = match request {
        RepositoryFixtureRequest::Initialize { root } => (root, &["init", "--quiet"]),
        RepositoryFixtureRequest::Status { root } => (
            root,
            &[
                "-c",
                "core.fsmonitor=false",
                "-c",
                "core.untrackedCache=false",
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
            ],
        ),
    };
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()
        .map_err(|_| RepositoryFixtureError::Spawn)?;
    if !output.status.success() {
        return Err(RepositoryFixtureError::Rejected);
    }
    if arguments.first() == Some(&"init") {
        Ok(RepositoryFixtureResponse::Initialized)
    } else {
        Ok(RepositoryFixtureResponse::Status(output.stdout))
    }
}

pub(super) fn initialize(root: &Path) -> Result<(), RepositoryFixtureError> {
    match execute(RepositoryFixtureRequest::Initialize { root })? {
        RepositoryFixtureResponse::Initialized => Ok(()),
        RepositoryFixtureResponse::Status(_) => Err(RepositoryFixtureError::UnexpectedResponse),
    }
}

pub(super) fn status(root: &Path) -> Result<Vec<u8>, RepositoryFixtureError> {
    match execute(RepositoryFixtureRequest::Status { root })? {
        RepositoryFixtureResponse::Status(bytes) => Ok(bytes),
        RepositoryFixtureResponse::Initialized => Err(RepositoryFixtureError::UnexpectedResponse),
    }
}
