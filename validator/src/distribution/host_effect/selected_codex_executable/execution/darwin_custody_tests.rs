use super::{HostEffectExecutorErrorId, session, spawn};
use crate::distribution::HostCommand;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn panic_boundary_requires_explicit_child_scope_finalization() {
    let cwd = File::open(".").expect("test cwd");
    let command = HostCommand::from_untrusted_record(
        "codex".to_owned(),
        vec!["--version".to_owned()],
        Vec::new(),
        1_000,
        1,
    );
    let spawned = spawn::spawn(std::path::Path::new("/bin/sh"), &command, cwd.as_raw_fd())
        .expect("suspended child");
    let session = session::ChildSession::new(spawned);
    let panic_result = catch_unwind(AssertUnwindSafe(|| panic!("panic control")));
    assert!(panic_result.is_err());

    let failure = session.finalize_failure(HostEffectExecutorErrorId::ProcessFailed);
    assert!(failure.started);
}
