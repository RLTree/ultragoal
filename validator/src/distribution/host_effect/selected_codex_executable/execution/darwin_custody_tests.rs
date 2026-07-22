use super::*;
use std::fs::File;
use std::os::fd::AsRawFd;

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
    let mut session = ChildSession::new(spawned);
    let panic_result = catch_unwind(AssertUnwindSafe(|| panic!("panic control")));
    assert!(panic_result.is_err());

    let failure = session.failure(HostEffectExecutorErrorId::ProcessFailed);
    assert!(failure.started);
    assert!(session.finalized);
    assert!(!recovery::group_exists(session.pid));
}
