use super::capture::{CommandSpec, PublicArg, PublicArtifact, PublicEnv, SecretArg};
use super::fixture::{RepoFixture, read_command};
use crate::context::EffectClass;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(unix)]
#[test]
fn symlink_and_hardlink_programs_are_never_opened_when_execution_is_disabled() {
    let fixture = RepoFixture::new("program-links");
    fixture.write_script("bin/real", "exit 0");
    symlink("real", fixture.root().join("bin/link")).unwrap();
    fs::hard_link(
        fixture.root().join("bin/real"),
        fixture.root().join("bin/hard"),
    )
    .unwrap();
    let context = fixture.context();
    for program in ["bin/link", "bin/hard", "bin/real"] {
        assert!(
            read_command(program)
                .run(&context)
                .unwrap_err()
                .contains("accepted typed-catalog binding")
        );
    }
}

#[cfg(unix)]
#[test]
fn symlink_cwd_is_not_followed() {
    let fixture = RepoFixture::new("cwd-link");
    fixture.write_script("bin/noop", "exit 0");
    fs::create_dir(fixture.root().join("real-cwd")).unwrap();
    symlink("real-cwd", fixture.root().join("linked-cwd")).unwrap();
    let error = read_command("bin/noop")
        .cwd("linked-cwd")
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
}

#[cfg(unix)]
#[test]
fn program_path_swap_surface_is_not_opened_before_fail_closed_rejection() {
    let fixture = RepoFixture::new("program-swap");
    fixture.write_script("bin/replacement", "printf PWNED > marker");
    symlink("replacement", fixture.root().join("bin/target")).unwrap();
    let error = read_command("bin/target")
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
    assert!(!fixture.root().join("marker").exists());
}

#[test]
fn traversal_shell_text_and_prohibited_worktree_paths_are_rejected() {
    let fixture = RepoFixture::new("path-denial");
    fixture.write_script("bin/noop", "exit 0");
    let context = fixture.context();
    assert!(read_command("../bin/noop").run(&context).is_err());
    assert!(read_command("/bin/sh").run(&context).is_err());
    assert!(read_command("bin/noop; echo PASS").run(&context).is_err());
    assert!(
        read_command("bin/noop")
            .public_artifact(PublicArtifact::new("../outside"))
            .run(&context)
            .is_err()
    );
    assert!(
        read_command(".codex-worktree/env.sh")
            .run(&context)
            .unwrap_err()
            .contains("prohibited secret-bearing")
    );
    assert!(
        read_command("bin/noop")
            .public_artifact(PublicArtifact::new("nested/.codex-worktree/env.sh"))
            .run(&context)
            .unwrap_err()
            .contains("prohibited secret-bearing")
    );
}

#[test]
fn structural_effect_mismatch_is_denied_before_substrate_selection() {
    let fixture = RepoFixture::new("effect-denial");
    fixture.write_script("bin/noop", "exit 0");
    let error = CommandSpec::new("bin/noop", EffectClass::WorkspaceWrite)
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("only Read no-spawn observations"));
}

#[test]
fn secret_values_cannot_cross_into_public_argument_or_environment_channels() {
    let fixture = RepoFixture::new("secret-channel");
    fixture.write_script("bin/noop", "exit 0");
    let context = fixture.context_with_secret("BOUND", "version-9");
    let canary = "supersecret-311";
    let error = read_command("bin/noop")
        .secret_arg(SecretArg::new(canary, "BOUND"))
        .public_arg(PublicArg::new(format!("prefix-{canary}-suffix")))
        .run(&context)
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
    assert!(!error.contains(canary));
    let error = read_command("bin/noop")
        .secret_arg(SecretArg::new(canary, "BOUND"))
        .public_environment(PublicEnv::new("VISIBLE", canary))
        .run(&context)
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
    assert!(!error.contains(canary));
}

#[test]
fn pre_interruption_remains_catalog_unbound() {
    let fixture = RepoFixture::new("binding");
    fixture.write_script("bin/noop", "exit 0");
    let error = read_command("bin/noop")
        .with_interrupt_flag(Arc::new(AtomicBool::new(true)))
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
}

#[test]
fn hidden_write_program_is_rejected_and_original_bytes_remain() {
    let fixture = RepoFixture::new("hidden-write");
    fixture.write_file("nested/original", b"original");
    fixture.write_script(
        "bin/mutate",
        "printf changed > nested/original; printf x > nested/transient",
    );
    let error = read_command("bin/mutate")
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
    assert_eq!(
        fs::read(fixture.root().join("nested/original")).unwrap(),
        b"original"
    );
    assert!(!fixture.root().join("nested/transient").exists());
}
