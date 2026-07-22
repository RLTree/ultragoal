use super::{SelectedCodexExecutableTestFixture, selected_test_fixture};
use std::fs;
use std::os::darwin::fs::MetadataExt as DarwinMetadataExt;
use std::os::unix::fs::MetadataExt;

#[test]
fn launch_uses_a_fresh_immutable_owner_only_copy() {
    let bytes = b"#!/bin/sh\nprintf sealed\n";
    let mut fixture = selected_test_fixture("darwin-copy", bytes);
    let selected = fixture.take_selected();
    let launch = selected.launch_path().to_owned();
    assert_ne!(launch, fixture.path.as_path());
    assert_eq!(fs::read(&launch).expect("sealed bytes"), bytes);
    let directory = launch.parent().expect("sealed directory");
    let directory_metadata = fs::symlink_metadata(directory).expect("sealed directory metadata");
    let launch_metadata = fs::symlink_metadata(&launch).expect("sealed file metadata");
    assert!(directory_metadata.is_dir());
    assert_eq!(directory_metadata.mode() & 0o7777, 0o700);
    assert_eq!(directory_metadata.uid(), unsafe { libc::geteuid() });
    assert_ne!(directory_metadata.st_flags() & libc::UF_IMMUTABLE, 0);
    assert!(launch_metadata.is_file());
    assert_eq!(launch_metadata.mode() & 0o7777, 0o700);
    assert_eq!(launch_metadata.uid(), unsafe { libc::geteuid() });
    assert_ne!(launch_metadata.st_flags() & libc::UF_IMMUTABLE, 0);
    selected.revalidate_launch().expect("sealed digest");
    assert!(fs::write(&launch, b"tampered").is_err());
    selected
        .revalidate_launch()
        .expect("immutable copy remains intact");
    let alias = selected.duplicate().expect("selected alias");
    drop(alias);
    assert!(fs::symlink_metadata(&launch).is_ok());
    selected.finalize().expect("explicit sealed cleanup");
    assert!(fs::symlink_metadata(&launch).is_err());
}

#[test]
fn sandbox_boundary_blocks_forking_controls() {
    let output = std::process::Command::new("/usr/bin/sandbox-exec")
        .args([
            "-p",
            "(version 1) (allow default) (deny process-fork (with send-signal SIGKILL))",
            "/usr/bin/perl",
            "-e",
            "fork(); sleep 60",
        ])
        .output()
        .expect("sandbox-exec availability");
    assert!(!output.status.success());
}

#[test]
fn executor_runs_sealed_copy_inside_contained_scope() {
    use std::os::fd::AsRawFd;

    let mut fixture: SelectedCodexExecutableTestFixture =
        selected_test_fixture("darwin-execute", b"#!/bin/sh\nprintf sealed-copy\n");
    let selected = fixture.take_selected();
    let capability = crate::distribution::host_effect::lifecycle::DescriptorExecutionCapability::new(
        crate::distribution::host_effect::lifecycle::DescriptorExecutionPlatform::Darwin,
        crate::distribution::host_effect::lifecycle::DescriptorExecutionPrimitive::DarwinPosixSpawnSuspendedLoadedVnode,
        "test-darwin-copy".to_owned(),
        "v1".to_owned(),
    )
    .expect("Darwin capability");
    let command = crate::distribution::HostCommand::from_untrusted_record(
        "codex".to_owned(),
        vec!["--version".to_owned()],
        Vec::new(),
        30_000,
        1,
    );
    let policy =
        crate::distribution::host_effect::executor::HostEffectExecutionPolicy::strict(30_000, &[])
            .expect("execution policy");
    let cancellation =
        crate::distribution::host_effect::executor::HostEffectCancellation::default();
    let cwd = fs::File::open(&fixture.root).expect("cwd");
    let capture = selected
        .execute(
            &capability,
            &command,
            &policy,
            &cancellation,
            cwd.as_raw_fd(),
        )
        .unwrap_or_else(|_| panic!("copied Darwin execution"));
    assert_eq!(capture.exit_code(), 0);
    assert_eq!(capture.stdout(), b"sealed-copy");
    selected.finalize().expect("explicit sealed cleanup");
}
