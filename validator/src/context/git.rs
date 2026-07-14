use super::bound_context::CandidateIdentity;
use super::digest::{add_framed, sha256_hex};
use super::error::{ContextError, io_error};
use super::process::{run_bounded, run_bounded_allow_failure};
use sha2::{Digest, Sha256};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const GIT_TIMEOUT: Duration = Duration::from_secs(15);

fn git_args(arguments: &[&str]) -> Vec<OsString> {
    [
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.untrackedCache=false",
        "-c",
        "diff.external=",
    ]
    .into_iter()
    .chain(arguments.iter().copied())
    .map(OsString::from)
    .collect()
}

fn run(git: &Path, cwd: &Path, arguments: &[&str]) -> Result<Vec<u8>, ContextError> {
    Ok(run_bounded(git, &git_args(arguments), cwd, GIT_TIMEOUT)?.stdout)
}

fn optional(git: &Path, cwd: &Path, arguments: &[&str]) -> Result<Option<Vec<u8>>, ContextError> {
    let output = run_bounded_allow_failure(git, &git_args(arguments), cwd, GIT_TIMEOUT)?;
    Ok(output.success.then_some(output.stdout))
}

fn path_from_bytes(bytes: Vec<u8>) -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from(OsString::from_vec(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(&bytes).into_owned())
    }
}

fn os_bytes(value: &OsStr) -> Vec<u8> {
    #[cfg(unix)]
    {
        value.as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        value.to_string_lossy().as_bytes().to_vec()
    }
}

fn canonical(path: PathBuf) -> Result<PathBuf, ContextError> {
    path.canonicalize().map_err(|error| io_error(path, error))
}

fn checked_relative(bytes: &[u8]) -> Result<PathBuf, ContextError> {
    let path = path_from_bytes(bytes.to_vec());
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(ContextError::PathDenied(format!(
            "Git returned non-relative path {}",
            path.display()
        )));
    }
    Ok(path)
}

pub(crate) fn resolve_roots(start: &Path, git: &Path) -> Result<(PathBuf, PathBuf), ContextError> {
    let start = canonical(start.to_path_buf())?;
    if !start.is_dir() {
        return Err(ContextError::InvalidRequest(format!(
            "context start must be a directory: {}",
            start.display()
        )));
    }
    let output = run_bounded_allow_failure(
        git,
        &git_args(&["rev-parse", "--show-toplevel"]),
        &start,
        GIT_TIMEOUT,
    )?;
    if !output.success {
        return Err(ContextError::NotGitRepository(start));
    }
    let worktree = canonical(path_from_bytes(trim_line(output.stdout)))?;
    let listing = run(git, &worktree, &["worktree", "list", "--porcelain", "-z"])?;
    let main = listing
        .split(|byte| *byte == 0)
        .find_map(|line| line.strip_prefix(b"worktree "))
        .ok_or_else(|| ContextError::Probe {
            program: "git worktree list".to_owned(),
            message: "did not identify a repository root".to_owned(),
        })?;
    let repository = canonical(path_from_bytes(main.to_vec()))?;
    Ok((repository, worktree))
}

fn trim_line(mut bytes: Vec<u8>) -> Vec<u8> {
    while matches!(bytes.last(), Some(b'\n' | b'\r')) {
        bytes.pop();
    }
    bytes
}

fn text(value: Option<Vec<u8>>, field: &str) -> Result<Option<String>, ContextError> {
    value
        .map(trim_line)
        .map(|bytes| {
            String::from_utf8(bytes).map_err(|_| {
                ContextError::InvalidRequest(format!("Git {field} identity is not UTF-8"))
            })
        })
        .transpose()
}

fn add_file(hasher: &mut Sha256, path: &Path, relative: &[u8]) -> Result<(), ContextError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    add_framed(hasher, relative);
    #[cfg(unix)]
    hasher.update(metadata.permissions().mode().to_be_bytes());
    #[cfg(not(unix))]
    hasher.update([metadata.permissions().readonly() as u8]);
    if metadata.file_type().is_symlink() {
        hasher.update(b"symlink");
        let target = fs::read_link(path).map_err(|error| io_error(path, error))?;
        add_framed(hasher, &os_bytes(target.as_os_str()));
    } else if metadata.is_file() {
        hasher.update(b"file");
        hasher.update(metadata.len().to_be_bytes());
        let mut file = File::open(path).map_err(|error| io_error(path, error))?;
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|error| io_error(path, error))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
    } else {
        hasher.update(b"special");
    }
    Ok(())
}

fn untracked_digest(git: &Path, root: &Path) -> Result<String, ContextError> {
    let listing = run(
        git,
        root,
        &["ls-files", "--others", "--exclude-standard", "-z", "--"],
    )?;
    let mut paths = listing
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    paths.sort();
    let mut hasher = Sha256::new();
    for bytes in paths {
        let relative = checked_relative(&bytes)?;
        add_file(&mut hasher, &root.join(relative), &bytes)?;
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn capture_candidate(
    git: &Path,
    root: &Path,
) -> Result<CandidateIdentity, ContextError> {
    let status = run(
        git,
        root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    let worktree_diff = run(
        git,
        root,
        &["diff", "--binary", "--no-ext-diff", "--no-textconv", "--"],
    )?;
    let staged_diff = run(
        git,
        root,
        &[
            "diff",
            "--cached",
            "--binary",
            "--no-ext-diff",
            "--no-textconv",
            "--",
        ],
    )?;
    let branch = text(
        optional(git, root, &["symbolic-ref", "--quiet", "--short", "HEAD"])?,
        "branch",
    )?;
    Ok(CandidateIdentity {
        head_commit: text(
            optional(git, root, &["rev-parse", "--verify", "HEAD^{commit}"])?,
            "HEAD commit",
        )?,
        head_tree: text(
            optional(git, root, &["rev-parse", "--verify", "HEAD^{tree}"])?,
            "HEAD tree",
        )?,
        branch,
        status_sha256: sha256_hex(&status),
        worktree_diff_sha256: sha256_hex(&worktree_diff),
        staged_diff_sha256: sha256_hex(&staged_diff),
        untracked_content_sha256: untracked_digest(git, root)?,
        dirty: !status.is_empty(),
    })
}
