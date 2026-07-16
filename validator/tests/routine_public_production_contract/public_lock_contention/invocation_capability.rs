mod capability_pipe;

use super::super::scenario::{Fixture, sha};
use serde::{Deserialize, Serialize};
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{
    fs::{self, File},
    io::{Read, Write},
};

const FD: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_CAPABILITY_FD";
const MODE: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_SUPERVISOR";
const CASE: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_CASE";
const ROOT: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_ROOT";
const HOME: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_HOME";
const BINARY: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_BINARY";
const VERSION: &str = "routine-public-supervisor-capability-v1";

#[derive(Clone, Copy, Debug)]
pub(super) enum CapabilityMode {
    Valid,
    Omit,
    WrongFd,
    Malformed,
    Duplicate,
    Trailing,
    WrongParent,
    WrongTest,
    WrongCase,
    WrongRoot,
    WrongHome,
    WrongBinary,
}

pub(super) struct ParentCapability(Option<File>);

impl ParentCapability {
    pub(super) fn close_after_spawn(self) {
        drop(self.0);
    }
}

pub(super) struct Invocation {
    pub case: String,
    pub root: PathBuf,
    pub home: PathBuf,
    pub binary: PathBuf,
}

#[derive(Deserialize, Serialize)]
struct Envelope {
    version: String,
    nonce: String,
    invocation: String,
    parent_pid: u32,
    test_name: String,
    case: String,
    root: Identity,
    home: Identity,
    binary: Identity,
}

#[derive(Deserialize, PartialEq, Serialize)]
struct Identity {
    path: String,
    device: u64,
    inode: u64,
    mode: u32,
    digest: Option<String>,
}

pub(super) fn issue(
    command: &mut Command,
    fixture: &Fixture,
    test_name: &str,
    case: &str,
    mode: CapabilityMode,
) -> ParentCapability {
    command
        .env(MODE, "routine-public-v1")
        .env(CASE, case)
        .env(ROOT, &fixture.root)
        .env(HOME, &fixture.home)
        .env(BINARY, fixture.binary_path());
    if matches!(mode, CapabilityMode::Omit) {
        return ParentCapability(None);
    }
    if matches!(mode, CapabilityMode::WrongFd) {
        command.env(FD, "0");
        return ParentCapability(None);
    }
    let mut envelope = envelope(fixture, test_name, case).unwrap();
    alter(&mut envelope, mode);
    let mut bytes = serde_json::to_vec(&envelope).unwrap();
    match mode {
        CapabilityMode::Malformed => bytes = b"{".to_vec(),
        CapabilityMode::Duplicate => bytes.extend_from_slice(&bytes.clone()),
        CapabilityMode::Trailing => bytes.push(b'x'),
        _ => {}
    }
    let (reader, mut writer) = capability_pipe::new_reader_writer();
    writer.write_all(&bytes).unwrap();
    drop(writer);
    command.env(FD, reader.as_raw_fd().to_string());
    ParentCapability(Some(reader))
}

pub(super) fn consume(expected_test: &str) -> Result<Option<Invocation>, String> {
    let Some(raw_fd) = std::env::var_os(FD) else {
        return Ok(None);
    };
    let fd = raw_fd
        .to_string_lossy()
        .parse::<i32>()
        .map_err(|_| "capability fd was malformed".to_owned())?;
    let bytes = capability_pipe::read_once(fd)?;
    let envelope: Envelope = serde_json::from_slice(&bytes)
        .map_err(|_| "capability envelope was malformed or trailing".to_owned())?;
    validate_envelope(&envelope, expected_test)?;
    let root = verify_env(ROOT, &envelope.root, false)?;
    let home = verify_env(HOME, &envelope.home, false)?;
    let binary = verify_env(BINARY, &envelope.binary, true)?;
    if std::env::var(MODE).as_deref() != Ok("routine-public-v1")
        || std::env::var(CASE).as_deref() != Ok(envelope.case.as_str())
    {
        return Err("capability selector did not match the sealed envelope".to_owned());
    }
    Ok(Some(Invocation {
        case: envelope.case,
        root,
        home,
        binary,
    }))
}

fn envelope(fixture: &Fixture, test_name: &str, case: &str) -> Result<Envelope, String> {
    let nonce = nonce();
    let parent_pid = std::process::id();
    Ok(Envelope {
        version: VERSION.to_owned(),
        invocation: format!("{parent_pid}:{nonce}"),
        nonce,
        parent_pid,
        test_name: test_name.to_owned(),
        case: case.to_owned(),
        root: identity(&fixture.root, false)?,
        home: identity(&fixture.home, false)?,
        binary: identity(fixture.binary_path(), true)?,
    })
}

fn alter(envelope: &mut Envelope, mode: CapabilityMode) {
    match mode {
        CapabilityMode::WrongParent => envelope.parent_pid += 1,
        CapabilityMode::WrongTest => envelope.test_name.push_str(".wrong"),
        CapabilityMode::WrongCase => envelope.case.push_str(".wrong"),
        CapabilityMode::WrongRoot => envelope.root.path = "/tmp/not-routine-fixture".to_owned(),
        CapabilityMode::WrongHome => envelope.home.path = "/tmp/not-routine-fixture".to_owned(),
        CapabilityMode::WrongBinary => envelope.binary.path = "/tmp/not-routine-fixture".to_owned(),
        _ => {}
    }
}

fn validate_envelope(envelope: &Envelope, expected_test: &str) -> Result<(), String> {
    if envelope.version != VERSION
        || envelope.test_name != expected_test
        || envelope.parent_pid != unsafe { libc::getppid() as u32 }
        || envelope.nonce.len() != 64
        || !envelope.nonce.bytes().all(|byte| byte.is_ascii_hexdigit())
        || envelope.invocation != format!("{}:{}", envelope.parent_pid, envelope.nonce)
    {
        return Err("capability envelope binding was refused".to_owned());
    }
    let parent = fixture_parent()?;
    for identity in [&envelope.root, &envelope.home, &envelope.binary] {
        if !Path::new(&identity.path).starts_with(&parent) {
            return Err("capability path escaped the fixture parent".to_owned());
        }
    }
    Ok(())
}

fn verify_env(name: &str, expected: &Identity, digest: bool) -> Result<PathBuf, String> {
    let raw = std::env::var_os(name).ok_or_else(|| "capability path was absent".to_owned())?;
    let path = fs::canonicalize(raw)
        .map_err(|_| "capability path could not be canonicalized".to_owned())?;
    let actual = identity(&path, digest)?;
    if !path.starts_with(fixture_parent()?) || &actual != expected {
        return Err("capability path identity did not match the envelope".to_owned());
    }
    Ok(path)
}

fn identity(path: &Path, digest: bool) -> Result<Identity, String> {
    let path =
        fs::canonicalize(path).map_err(|_| "capability identity was unavailable".to_owned())?;
    let metadata =
        fs::metadata(&path).map_err(|_| "capability metadata was unavailable".to_owned())?;
    Ok(Identity {
        path: path.to_string_lossy().into_owned(),
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        digest: digest
            .then(|| fs::read(&path).map(|bytes| sha(&bytes)))
            .transpose()
            .map_err(|_| "capability binary digest was unavailable".to_owned())?,
    })
}

fn fixture_parent() -> Result<PathBuf, String> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .parent()
        .ok_or_else(|| "fixture workspace parent was absent".to_owned())?;
    fs::canonicalize(workspace.join("target/routine-public-contract-fixtures"))
        .map_err(|_| "fixture parent was absent".to_owned())
}

fn nonce() -> String {
    let mut bytes = [0_u8; 32];
    File::open("/dev/urandom")
        .unwrap()
        .read_exact(&mut bytes)
        .unwrap();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
