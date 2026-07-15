use super::super::scenario::{Fixture, sha};
use super::supervisor::{bound_paths, lock, public_command};
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::process::{Child, Output};

const FRAME_PREFIX: &str = "HUL_ROUTINE_PUBLIC_LIVE_CHILD_V1:";
const FRAME_VERSION: &str = "routine-public-live-child-v1";
pub(super) const PANIC_SENTINEL: &str =
    "routine-public-live-child-v1: verified configured product child is stopped";

pub(super) fn panic_with_live_child(case: &str) -> ! {
    let (root, home, binary) = bound_paths();
    if case == "panic-live-child-spawn-refusal" {
        spawn_refusal(&root, &home, &binary);
    }
    let _holder = lock(&home);
    let mut command = public_command(&root, &home, &binary);
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("live-child-handshake-v1 spawn-failure: {error}"));
    if case == "panic-live-child-early-exit" {
        stop_and_reap(&mut child, "early-exit");
        panic!("live-child-handshake-v1 early-exit");
    }
    if case == "panic-live-child-signal-failure" {
        let child_id = child.id();
        stop_and_reap(&mut child, "signal-failure");
        assert_ne!(
            unsafe { libc::kill(child_id as i32, libc::SIGSTOP) },
            0,
            "live-child-handshake-v1 signal-failure unexpectedly stopped a reused pid"
        );
        panic!("live-child-handshake-v1 signal-failure");
    }
    let expected_group = current_group();
    let first_group = live_group(&mut child);
    if case == "panic-live-child-wrong-group" {
        require_group(first_group, expected_group.checked_add(1).unwrap());
    }
    require_group(first_group, expected_group);
    assert_eq!(
        unsafe { libc::kill(child.id() as i32, libc::SIGSTOP) },
        0,
        "live-child-handshake-v1 stop-failure"
    );
    require_group(live_group(&mut child), expected_group);
    emit_frame(case, &binary, expected_group);
    if case == "panic-live-child-wrong-sentinel" {
        panic!("live-child-handshake-v1 wrong panic sentinel");
    }
    panic!("{PANIC_SENTINEL}");
}

pub(super) fn verify_live_child_panic(
    output: &Output,
    fixture: &Fixture,
    expected_group: i32,
) -> Result<(), String> {
    if output.status.success() {
        return Err("live-child handshake supervisor unexpectedly succeeded".to_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.matches(PANIC_SENTINEL).count() != 1 {
        return Err("live-child handshake has no exact panic sentinel".to_owned());
    }
    let stdout = std::str::from_utf8(&output.stdout)
        .map_err(|_| "live-child handshake stdout was not UTF-8".to_owned())?;
    let frames: Vec<_> = stdout
        .lines()
        .flat_map(|line| {
            line.match_indices(FRAME_PREFIX)
                .map(|(index, _)| &line[index + FRAME_PREFIX.len()..])
        })
        .collect();
    let [frame] = frames.as_slice() else {
        return Err("live-child handshake was missing or duplicated".to_owned());
    };
    let frame: Value = serde_json::from_str(frame)
        .map_err(|_| "live-child handshake frame was malformed".to_owned())?;
    let canonical = fs::canonicalize(fixture.binary_path())
        .map_err(|_| "fixture binary could not be canonicalized".to_owned())?;
    let expected_path = canonical.to_string_lossy();
    let expected_digest =
        sha(&fs::read(&canonical).map_err(|_| "fixture binary could not be read".to_owned())?);
    if frame["version"] != FRAME_VERSION
        || frame["binary"] != expected_path.as_ref()
        || frame["digest"] != expected_digest
        || frame["group"].as_i64() != Some(i64::from(expected_group))
    {
        return Err(
            "live-child handshake did not bind fixture identity and supervisor group".to_owned(),
        );
    }
    Ok(())
}

fn spawn_refusal(root: &std::path::Path, home: &std::path::Path, binary: &std::path::Path) -> ! {
    let missing = binary.with_file_name("missing-routine-public-binary");
    match public_command(&root.to_path_buf(), &home.to_path_buf(), &missing).spawn() {
        Err(_) => panic!("live-child-handshake-v1 spawn-refusal for wrong configured binary"),
        Ok(mut child) => {
            stop_and_reap(&mut child, "spawn-refusal-unexpected-child");
            panic!("live-child-handshake-v1 spawn-refusal unexpectedly spawned");
        }
    }
}

fn current_group() -> i32 {
    let group = unsafe { libc::getpgrp() };
    assert!(
        group > 0,
        "live-child-handshake-v1 supervisor group unavailable"
    );
    group
}

fn live_group(child: &mut Child) -> i32 {
    assert!(
        child.try_wait().unwrap().is_none(),
        "live-child-handshake-v1 child exited before observation"
    );
    assert_eq!(
        unsafe { libc::kill(child.id() as i32, 0) },
        0,
        "live-child-handshake-v1 liveness failure"
    );
    let group = unsafe { libc::getpgid(child.id() as i32) };
    assert!(group > 0, "live-child-handshake-v1 child group unavailable");
    group
}

fn require_group(observed: i32, expected: i32) {
    if observed != expected {
        panic!("live-child-handshake-v1 wrong-group");
    }
}

fn stop_and_reap(child: &mut Child, context: &str) {
    assert_eq!(
        unsafe { libc::kill(child.id() as i32, libc::SIGKILL) },
        0,
        "live-child-handshake-v1 {context} kill failed"
    );
    child
        .wait()
        .unwrap_or_else(|error| panic!("live-child-handshake-v1 {context} reap failed: {error}"));
}

fn emit_frame(case: &str, binary: &std::path::Path, group: i32) {
    let canonical = fs::canonicalize(binary).unwrap();
    let mut frame = json!({
        "version": FRAME_VERSION,
        "binary": canonical.to_string_lossy(),
        "digest": sha(&fs::read(&canonical).unwrap()),
        "group": group,
    });
    if case == "panic-live-child-wrong-binary" {
        frame["binary"] = json!(canonical.with_file_name("wrong-routine-public-binary"));
    }
    match case {
        "panic-live-child-missing-handshake" => {}
        "panic-live-child-malformed-handshake" => println!("{FRAME_PREFIX}{{"),
        "panic-live-child-duplicate-handshake" => {
            println!("{FRAME_PREFIX}{frame}");
            println!("{FRAME_PREFIX}{frame}");
        }
        _ => println!("{FRAME_PREFIX}{frame}"),
    }
    std::io::stdout().flush().unwrap();
}
