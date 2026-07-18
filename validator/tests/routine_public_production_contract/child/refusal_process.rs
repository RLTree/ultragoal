use super::super::scenario::{Fixture, pass_node, prefix_route, sha};
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::process::{Output, Stdio};
use std::time::{Duration, Instant};
use ultragoal::routine_work::{RustSourceFrameInput, encode_rust_source_syntax_frame};

pub(super) const CHILD_CHANNEL_FD: i32 = 198;

pub(super) fn run_with_prebuffered_channel(fixture: &Fixture, payload: &[u8]) -> Output {
    let (mut peer, child_endpoint) = UnixStream::pair().unwrap();
    peer.write_all(payload).unwrap();
    let source_fd = child_endpoint.as_raw_fd();
    let mut command = fixture.base_command();
    command
        .args(["--json", "check", "routine"])
        .env("HUL_ROUTINE_CHILD_FD", CHILD_CHANNEL_FD.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(source_fd, CHILD_CHANNEL_FD) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let flags = libc::fcntl(CHILD_CHANNEL_FD, libc::F_GETFD);
            if flags < 0
                || libc::fcntl(CHILD_CHANNEL_FD, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let output = run_with_frame(command, frame(fixture));
    drop(child_endpoint);
    output
}

pub(super) fn run_with_blocking_socket_and_stdin(fixture: &Fixture) -> Output {
    let (_peer, child_endpoint) = UnixStream::pair().unwrap();
    let source_fd = child_endpoint.as_raw_fd();
    let mut command = fixture.base_command();
    command
        .args(["--json", "check", "routine"])
        .env("HUL_ROUTINE_CHILD_FD", CHILD_CHANNEL_FD.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(source_fd, CHILD_CHANNEL_FD) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let flags = libc::fcntl(CHILD_CHANNEL_FD, libc::F_GETFD);
            if flags < 0
                || libc::fcntl(CHILD_CHANNEL_FD, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_millis(750);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("child request blocked while reading socket or stdin");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    child.wait_with_output().unwrap()
}

pub(super) fn legacy_capability_wire() -> Vec<u8> {
    let body = br#"{"schema_version":"RoutineChildCapability-v1","behavior_id":"rust-source-syntax-v1","request_id":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","protocol_id":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","intent_id":"attacker-intent","node_id":"attacker-node","grant_session_id":"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","grant_id":"sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd","reservation_marker":"sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","parent_pid":2,"child_pid":3,"process_session_id":4,"program_path_hex":"2f62696e2f7368","program_sha256":"sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff","program_byte_length":1,"program_unix_mode":33061,"program_device":1,"program_inode":1,"program_changed_seconds":0,"program_changed_nanos":0,"framed_input_sha256":"sha256:1111111111111111111111111111111111111111111111111111111111111111","nonce_hex":"0000000000000000000000000000000000000000000000000000000000000000","capability_seal":"sha256:2222222222222222222222222222222222222222222222222222222222222222"}"#;
    let mut wire = vec![7_u8; 32];
    wire.extend_from_slice(&(body.len() as u32).to_be_bytes());
    wire.extend_from_slice(body);
    wire
}

pub(super) fn run_with_frame(mut command: std::process::Command, frame: Vec<u8>) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&frame);
    }
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

pub(super) fn assert_child_refused(output: &Output) {
    assert_refused(output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("RoutineBehaviorRefusal-v1"));
}
