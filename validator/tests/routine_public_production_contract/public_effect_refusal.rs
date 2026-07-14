use super::scenario::{Fixture, pass_node, prefix_route, tree};
use serde_json::Value;
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
