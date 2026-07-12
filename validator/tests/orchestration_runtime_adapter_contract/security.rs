use super::support::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use crate::runtime_adapter::*;
use std::fs::{self, OpenOptions};

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::os::unix::net::UnixListener;

#[cfg(unix)]
#[test]
fn path_escape_links_hardlinks_fifo_and_socket_roots_fail_closed() {
    let (root, engine) = durable_engine("security-paths", false);
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let escaped = root.path().join("..").join("security-path-canary");
    assert_eq!(
        ProductWorkspace::open(&escaped).unwrap_err(),
        ProductError::InvalidWorkspacePath
    );

    let link = root.path().with_extension("link");
    symlink(root.path(), &link).unwrap();
    assert_eq!(
        ProductWorkspace::open(&link).unwrap_err(),
        ProductError::InvalidWorkspacePath
    );
    fs::remove_file(&link).unwrap();

    let fifo = root.path().with_extension("fifo");
    let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
    assert_eq!(
        ProductWorkspace::open(&fifo).unwrap_err(),
        ProductError::InvalidWorkspacePath
    );
    fs::remove_file(&fifo).unwrap();

    let socket =
        std::env::temp_dir().join(format!("hul-runtime-adapter-{}.socket", std::process::id()));
    if socket.exists() {
        fs::remove_file(&socket).unwrap();
    }
    let listener = UnixListener::bind(&socket).unwrap();
    assert_eq!(
        ProductWorkspace::open(&socket).unwrap_err(),
        ProductError::InvalidWorkspacePath
    );
    drop(listener);
    fs::remove_file(&socket).unwrap();

    let outside = TestRoot::new("security-hardlink-holder");
    fs::hard_link(
        root.path().join("events.jsonl"),
        outside.path().join("events-link"),
    )
    .unwrap();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let context = context();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let before_len = fs::metadata(root.path().join("events.jsonl"))
        .unwrap()
        .len();
    let before_head = fs::read(root.path().join("head.json")).unwrap();
    assert!(matches!(
        adapter.inspect_current(&state_request(head, 1)),
        Err(ProductError::Kernel(OrchestrationError::JournalCorrupt))
    ));
    assert_eq!(
        fs::metadata(root.path().join("events.jsonl"))
            .unwrap()
            .len(),
        before_len
    );
    assert_eq!(
        fs::read(root.path().join("head.json")).unwrap(),
        before_head
    );
}

#[cfg(unix)]
#[test]
fn anchored_workspace_swap_refuses_without_reading_or_echoing_substitute() {
    let (root, engine) = durable_engine("security-root-swap-private-canary", false);
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let context = context();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let moved = root.path().with_extension("anchored");
    let outside = TestRoot::new("security-outside-private-canary");
    fs::rename(root.path(), &moved).unwrap();
    symlink(outside.path(), root.path()).unwrap();

    let error = adapter
        .inspect_current(&state_request(head, 1))
        .unwrap_err();
    assert_eq!(error, ProductError::WorkspaceChanged);
    assert!(!error.to_string().contains("private-canary"));
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());

    fs::remove_file(root.path()).unwrap();
    fs::rename(moved, root.path()).unwrap();
}

#[test]
fn oversized_journal_and_attacker_values_refuse_without_writes_or_echo() {
    let (root, engine) = durable_engine("security-oversized-private-canary", false);
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    OpenOptions::new()
        .write(true)
        .open(root.path().join("events.jsonl"))
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    let before_len = fs::metadata(root.path().join("events.jsonl"))
        .unwrap()
        .len();
    let before_head = fs::read(root.path().join("head.json")).unwrap();
    let context = context();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let error = adapter
        .inspect_current(&state_request(head, 1))
        .unwrap_err();
    assert!(matches!(
        error,
        ProductError::Kernel(OrchestrationError::JournalCorrupt)
    ));
    assert!(!error.to_string().contains("private-canary"));
    assert_eq!(
        fs::metadata(root.path().join("events.jsonl"))
            .unwrap()
            .len(),
        before_len
    );
    assert_eq!(
        fs::read(root.path().join("head.json")).unwrap(),
        before_head
    );
}
