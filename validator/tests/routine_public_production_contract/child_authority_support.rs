use super::super::scenario::{Fixture, pass_node, prefix_route, sha};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::process::{Output, Stdio};
use ultragoal::routine_work::{RustSourceFrameInput, encode_rust_source_syntax_frame};

pub(super) const CAPABILITY_FD: i32 = 198;

#[derive(Clone, Serialize)]
struct ForgedCapability {
    schema_version: &'static str,
    behavior_id: &'static str,
    request_id: String,
    protocol_id: String,
    intent_id: &'static str,
    node_id: &'static str,
    grant_session_id: String,
    grant_id: String,
    reservation_marker: String,
    parent_pid: i32,
    child_pid: i32,
    process_session_id: i32,
    program_path_hex: String,
    program_sha256: String,
    program_byte_length: u64,
    program_unix_mode: Option<u32>,
    program_device: u64,
    program_inode: u64,
    program_changed_seconds: i64,
    program_changed_nanos: i64,
    framed_input_sha256: String,
    nonce_hex: String,
    capability_seal: String,
}

#[derive(Serialize)]
struct SealPayload<'a> {
    domain: &'static str,
    capability: &'a ForgedCapability,
}

#[derive(Clone)]
pub(super) struct ForgedMaterial {
    secret: [u8; 32],
    bytes: Vec<u8>,
}

pub(super) fn run_with_forged_channel(
    fixture: &Fixture,
    wrong_session: bool,
    replay: Option<&ForgedMaterial>,
) -> (Output, ForgedMaterial) {
    let (mut parent, child_endpoint) = UnixStream::pair().unwrap();
    let source_fd = child_endpoint.as_raw_fd();
    let mut command = fixture.base_command();
    command
        .args(["--json", "check", "routine"])
        .env("HUL_ROUTINE_CHILD_FD", CAPABILITY_FD.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(source_fd, CAPABILITY_FD) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let flags = libc::fcntl(CAPABILITY_FD, libc::F_GETFD);
            if flags < 0 || libc::fcntl(CAPABILITY_FD, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    drop(child_endpoint);
    let input = frame(fixture);
    let material = replay
        .cloned()
        .unwrap_or_else(|| forged_material(&child, &input, wrong_session));
    parent.write_all(&material.secret).unwrap();
    parent
        .write_all(&(material.bytes.len() as u32).to_be_bytes())
        .unwrap();
    parent.write_all(&material.bytes).unwrap();
    child.stdin.take().unwrap().write_all(&input).unwrap();
    (child.wait_with_output().unwrap(), material)
}

fn forged_material(
    child: &std::process::Child,
    input: &[u8],
    wrong_session: bool,
) -> ForgedMaterial {
    let binary = Fixture::binary();
    let metadata = fs::metadata(&binary).unwrap();
    let request_id = sha(b"request");
    let protocol_id = sha(b"protocol");
    let grant_id = sha(b"grant");
    let mut capability = ForgedCapability {
        schema_version: "RoutineChildCapability-v1",
        behavior_id: "rust-source-syntax-v1",
        request_id: request_id.clone(),
        protocol_id: protocol_id.clone(),
        intent_id: "attacker-intent",
        node_id: "attacker-node",
        grant_session_id: sha(b"session"),
        grant_id: grant_id.clone(),
        reservation_marker: framed(&[
            b"routine-mediated-recovery-v1",
            grant_id.as_bytes(),
            protocol_id.as_bytes(),
            request_id.as_bytes(),
        ]),
        parent_pid: std::process::id() as i32,
        child_pid: child.id() as i32,
        process_session_id: unsafe { libc::getsid(0) } + i32::from(wrong_session),
        program_path_hex: binary
            .as_os_str()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        program_sha256: sha(&fs::read(&binary).unwrap()),
        program_byte_length: metadata.len(),
        program_unix_mode: Some(metadata.mode()),
        program_device: metadata.dev(),
        program_inode: metadata.ino(),
        program_changed_seconds: metadata.ctime(),
        program_changed_nanos: metadata.ctime_nsec(),
        framed_input_sha256: sha(input),
        nonce_hex: "00".repeat(32),
        capability_seal: String::new(),
    };
    let secret = [7_u8; 32];
    capability.seal(&secret);
    ForgedMaterial {
        secret,
        bytes: serde_json::to_vec(&capability).unwrap(),
    }
}

pub(super) fn run_with_frame(mut command: std::process::Command, frame: Vec<u8>) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&frame).unwrap();
    child.wait_with_output().unwrap()
}

pub(super) fn frame(fixture: &Fixture) -> Vec<u8> {
    let source = fs::read(fixture.root.join("src/lib.rs")).unwrap();
    encode_rust_source_syntax_frame(&[RustSourceFrameInput::new(
        "src/lib.rs",
        &sha(&source),
        source.len() as u64,
        &source,
    )])
    .unwrap()
}

pub(super) fn dirty_fixture(label: &str) -> Fixture {
    Fixture::new(
        label,
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        false,
    )
}

pub(super) fn assert_refused(output: &Output) {
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("RoutineBehaviorObservation-v1"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("RoutineBehaviorObservation-v1"));
}

impl ForgedCapability {
    fn seal(&mut self, secret: &[u8; 32]) {
        let mut unsealed = self.clone();
        unsealed.capability_seal.clear();
        let payload = serde_json::to_vec(&SealPayload {
            domain: "routine-child-capability-seal-v1",
            capability: &unsealed,
        })
        .unwrap();
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
        mac.update(&payload);
        self.capability_seal = format!("sha256:{:x}", mac.finalize().into_bytes());
    }
}

fn framed(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    format!("sha256:{:x}", hasher.finalize())
}
