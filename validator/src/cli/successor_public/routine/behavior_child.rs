use super::*;
use crate::cli::successor::ExitClass;
#[cfg(target_vendor = "apple")]
use crate::routine_work::{
    CHILD_CAPABILITY_ENV, CHILD_CAPABILITY_FD, MAX_CHILD_CAPABILITY_BYTES, RoutineChildCapability,
    RustSourceSyntaxOutcome, evaluate_rust_source_syntax_frame, immutable_routine_program_matches,
    rust_source_syntax_observation_json,
};
#[cfg(target_vendor = "apple")]
use sha2::{Digest, Sha256};
#[cfg(target_vendor = "apple")]
use std::ffi::OsString;
#[cfg(target_vendor = "apple")]
use std::io::{Read, Write};
#[cfg(target_vendor = "apple")]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(target_vendor = "apple")]
use std::os::unix::ffi::OsStringExt;
#[cfg(target_vendor = "apple")]
use std::os::unix::net::UnixStream;

#[cfg(target_vendor = "apple")]
const MAX_FRAME_BYTES: u64 = 16 * 1024 * 1024;

#[cfg(target_vendor = "apple")]
pub(crate) fn execute_if_requested(invocation: &ParsedInvocation) -> Option<RuntimeOutcome> {
    let descriptor = std::env::var(CHILD_CAPABILITY_ENV).ok()?;
    if descriptor != CHILD_CAPABILITY_FD.to_string() {
        return Some(refusal());
    }
    if unsafe { libc::fcntl(CHILD_CAPABILITY_FD, libc::F_GETFD) } < 0 {
        return Some(refusal());
    }
    Some(execute_authorized(invocation).unwrap_or_else(refusal))
}

#[cfg(not(target_vendor = "apple"))]
pub(crate) fn execute_if_requested(_invocation: &ParsedInvocation) -> Option<RuntimeOutcome> {
    std::env::var_os("HUL_ROUTINE_CHILD_FD").map(|_| refusal())
}

#[cfg(target_vendor = "apple")]
fn execute_authorized(invocation: &ParsedInvocation) -> Option<RuntimeOutcome> {
    let mut channel = unsafe { UnixStream::from_raw_fd(CHILD_CAPABILITY_FD) };
    let timeout = Some(std::time::Duration::from_secs(2));
    channel.set_read_timeout(timeout).ok()?;
    channel.set_write_timeout(timeout).ok()?;
    let peer_pid = peer_pid(&channel)?;
    let (capability, secret) = read_capability(&mut channel)?;
    capability.verify_seal(&secret).ok()?;
    validate_capability(invocation, &capability, peer_pid)?;
    channel
        .write_all(capability.acknowledgement().as_bytes())
        .ok()?;
    channel.shutdown(std::net::Shutdown::Both).ok()?;
    drop(channel);

    let mut frame = Vec::new();
    if std::io::stdin()
        .take(MAX_FRAME_BYTES + 1)
        .read_to_end(&mut frame)
        .is_err()
        || frame.len() as u64 > MAX_FRAME_BYTES
    {
        return None;
    }
    if format!("sha256:{:x}", Sha256::digest(&frame)) != capability.framed_input_sha256 {
        return None;
    }
    match evaluate_rust_source_syntax_frame(&frame) {
        RustSourceSyntaxOutcome::Passed(observation) => Some(RuntimeOutcome::payload(
            ExitClass::Success,
            rust_source_syntax_observation_json(&observation),
            "routine behavior observed".to_owned(),
        )),
        RustSourceSyntaxOutcome::Refused(_) => None,
    }
}

#[cfg(target_vendor = "apple")]
fn read_capability(channel: &mut UnixStream) -> Option<(RoutineChildCapability, [u8; 32])> {
    let mut secret = [0_u8; 32];
    channel.read_exact(&mut secret).ok()?;
    let mut length = [0_u8; 4];
    channel.read_exact(&mut length).ok()?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_CHILD_CAPABILITY_BYTES {
        return None;
    }
    let mut bytes = vec![0_u8; length];
    channel.read_exact(&mut bytes).ok()?;
    RoutineChildCapability::decode(&bytes)
        .ok()
        .map(|capability| (capability, secret))
}

#[cfg(target_vendor = "apple")]
fn validate_capability(
    invocation: &ParsedInvocation,
    capability: &RoutineChildCapability,
    peer_pid: i32,
) -> Option<()> {
    let parent_pid = unsafe { libc::getppid() };
    let child_pid = std::process::id() as i32;
    let session_id = unsafe { libc::getsid(0) };
    if capability.behavior_id != manifest::ROUTINE_BEHAVIOR
        || invocation.command != SuccessorCommand::Check(CheckProfile::Routine)
        || invocation.effect != EffectClass::WorkspaceWrite
        || !invocation.arguments.is_empty()
        || !process_binding_matches(
            capability,
            parent_pid,
            peer_pid,
            child_pid,
            session_id,
            unsafe { libc::getsid(parent_pid) },
        )
    {
        return None;
    }
    let program = immutable_routine_program_matches(
        &capability.program_path_hex,
        &capability.program_sha256,
        capability.program_byte_length,
        capability.program_unix_mode,
        capability.program_device,
        capability.program_inode,
        capability.program_changed_seconds,
        capability.program_changed_nanos,
    )
    .ok()?;
    let current = std::env::current_exe().ok()?;
    if current != program || process_path(parent_pid)? != program {
        return None;
    }
    Some(())
}

#[cfg(target_vendor = "apple")]
fn process_binding_matches(
    capability: &RoutineChildCapability,
    parent_pid: i32,
    peer_pid: i32,
    child_pid: i32,
    session_id: i32,
    parent_session_id: i32,
) -> bool {
    capability.parent_pid == parent_pid
        && capability.parent_pid == peer_pid
        && capability.child_pid == child_pid
        && capability.process_session_id == session_id
        && parent_session_id == session_id
}

#[cfg(target_vendor = "apple")]
fn peer_pid(channel: &UnixStream) -> Option<i32> {
    let mut pid = 0_i32;
    let mut length = std::mem::size_of::<i32>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            channel.as_raw_fd(),
            libc::SOL_LOCAL,
            libc::LOCAL_PEERPID,
            (&mut pid as *mut i32).cast(),
            &mut length,
        )
    };
    (result == 0 && length as usize == std::mem::size_of::<i32>()).then_some(pid)
}

#[cfg(target_vendor = "apple")]
fn process_path(pid: i32) -> Option<PathBuf> {
    let mut buffer = vec![0_u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    let length = unsafe {
        libc::proc_pidpath(
            pid,
            buffer.as_mut_ptr().cast(),
            buffer.len().try_into().ok()?,
        )
    };
    (length > 0).then(|| PathBuf::from(OsString::from_vec(buffer[..length as usize].to_vec())))
}

fn refusal() -> RuntimeOutcome {
    RuntimeOutcome::payload(
        ExitClass::ActionableFinding,
        br#"{"schema_version":"RoutineBehaviorRefusal-v1","behavior_id":"rust-source-syntax-v1","status":"refused"}"#.to_vec(),
        "routine behavior refused".to_owned(),
    )
}

#[cfg(all(test, target_vendor = "apple"))]
mod tests {
    use super::*;

    #[test]
    fn exact_internal_process_binding_is_accepted_but_parent_and_session_changes_refuse() {
        let capability = capability();
        assert!(process_binding_matches(&capability, 11, 11, 12, 13, 13));
        assert!(!process_binding_matches(&capability, 10, 11, 12, 13, 13));
        assert!(!process_binding_matches(&capability, 11, 10, 12, 13, 13));
        assert!(!process_binding_matches(&capability, 11, 11, 12, 14, 14));
        assert!(!process_binding_matches(&capability, 11, 11, 12, 13, 14));
    }

    fn capability() -> RoutineChildCapability {
        let digest = format!("sha256:{}", "0".repeat(64));
        RoutineChildCapability {
            schema_version: "RoutineChildCapability-v1".to_owned(),
            behavior_id: "rust-source-syntax-v1".to_owned(),
            request_id: digest.clone(),
            protocol_id: digest.clone(),
            intent_id: "intent".to_owned(),
            node_id: "node".to_owned(),
            grant_session_id: digest.clone(),
            grant_id: digest.clone(),
            reservation_marker: digest.clone(),
            parent_pid: 11,
            child_pid: 12,
            process_session_id: 13,
            program_path_hex: "2f62696e2f7368".to_owned(),
            program_sha256: digest,
            program_byte_length: 1,
            program_unix_mode: Some(0o100555),
            program_device: 1,
            program_inode: 1,
            program_changed_seconds: 0,
            program_changed_nanos: 0,
            framed_input_sha256: digest.clone(),
            nonce_hex: "0".repeat(64),
            capability_seal: digest,
        }
    }
}
