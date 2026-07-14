use super::scenario::{Fixture, git, pass_node, prefix_route, tree};
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Output;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn dirty_fixture(label: &str, provision_host: bool) -> Fixture {
    Fixture::new(
        label,
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        provision_host,
    )
}

fn assert_root_broker_refusal(output: &Output) {
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        diagnostic["diagnostic_id"],
        "successor_runtime_authority_required"
    );
    assert_eq!(diagnostic["cause"], "mediator-child-root-broker-required");
    assert_eq!(diagnostic["effect"], "none");
}

fn assert_fixture_unchanged(
    fixture: &Fixture,
    root: &std::collections::BTreeMap<String, String>,
    home: &std::collections::BTreeMap<String, String>,
    status: &[u8],
) {
    assert_eq!(tree(&fixture.root), *root);
    assert_eq!(tree(&fixture.home), *home);
    assert_eq!(fixture.status(), status);
    assert!(!fixture.root.join("target").exists());
}

#[test]
fn valid_host_refuses_before_output_or_authority_state_changes() {
    let fixture = dirty_fixture("broker-before-host-effect", true);
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    assert_eq!(
        std::fs::read_dir(fixture.authority_root()).unwrap().count(),
        0
    );

    assert_root_broker_refusal(&fixture.run());

    assert_fixture_unchanged(&fixture, &before_root, &before_home, &before_status);
    assert_eq!(
        std::fs::read_dir(fixture.authority_root()).unwrap().count(),
        0
    );
}

#[test]
fn missing_host_repeat_refusals_never_initialize_state_or_outputs() {
    let fixture = dirty_fixture("broker-repeat-zero-effect", false);
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    assert!(!fixture.state_root().exists());

    for _ in 0..4 {
        assert_root_broker_refusal(&fixture.run());
        assert_fixture_unchanged(&fixture, &before_root, &before_home, &before_status);
        assert!(!fixture.state_root().exists());
    }
}

#[test]
fn concurrent_refusals_never_initialize_or_reserve_authority() {
    let fixture = dirty_fixture("broker-concurrent-zero-effect", true);
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();

    std::thread::scope(|scope| {
        let done = Arc::new(AtomicBool::new(false));
        let monitor_done = Arc::clone(&done);
        let monitored = &fixture;
        let monitor = scope.spawn(move || {
            while !monitor_done.load(Ordering::Acquire) {
                assert!(!monitored.root.join("target").exists());
                assert_eq!(
                    std::fs::read_dir(monitored.authority_root())
                        .unwrap()
                        .count(),
                    0
                );
                std::thread::yield_now();
            }
        });
        let attempts = (0..8)
            .map(|_| scope.spawn(|| fixture.run()))
            .collect::<Vec<_>>();
        for attempt in attempts {
            assert_root_broker_refusal(&attempt.join().unwrap());
        }
        done.store(true, Ordering::Release);
        monitor.join().unwrap();
    });

    assert_fixture_unchanged(&fixture, &before_root, &before_home, &before_status);
    assert_eq!(
        std::fs::read_dir(fixture.authority_root()).unwrap().count(),
        0
    );
}

#[test]
fn public_refusal_does_not_enter_discovery_git_or_tool_probes() {
    let fixture = dirty_fixture("broker-before-process-spawn", true);
    let probe_dir = fixture.container.join("process-probes");
    let marker = fixture.container.join("process-spawned");
    fs::create_dir(&probe_dir).unwrap();
    for name in ["git", "ultragoal", "cargo", "rustc"] {
        let probe = probe_dir.join(name);
        fs::write(
            &probe,
            format!(
                "#!/bin/sh\n/usr/bin/touch '{}'\nexit 90\n",
                marker.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&probe, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let git_probe = probe_dir.join("git-fsmonitor");
    fs::write(
        &git_probe,
        format!("#!/bin/sh\n/usr/bin/touch '{}'\nexit 1\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&git_probe, fs::Permissions::from_mode(0o700)).unwrap();
    git(
        &fixture.root,
        &["config", "core.fsmonitor", git_probe.to_str().unwrap()],
    );
    let before_status = fixture.status();
    assert!(marker.exists(), "pinned Git status did not arm the probe");
    fs::remove_file(&marker).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);

    let mut command = fixture.base_command();
    let output = command
        .env("PATH", &probe_dir)
        .args(["--json", "check", "routine"])
        .output()
        .unwrap();

    assert_root_broker_refusal(&output);
    assert!(
        !marker.exists(),
        "public refusal entered discovery Git or a tool probe"
    );
    assert_fixture_unchanged(&fixture, &before_root, &before_home, &before_status);
    assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 0);
}
