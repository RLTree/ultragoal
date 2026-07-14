#[test]
fn external_callers_cannot_capture_construct_clone_or_deserialize_observation_frames() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = std::path::PathBuf::from("/tmp").join(format!(
        "hul-host-observation-privacy-probe-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/bin")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "host-lifecycle-privacy-probe"
version = "0.0.0"
edition = "2024"

[dependencies]
ultragoal = {{ path = "{}" }}
serde = {{ version = "1.0.228", features = ["derive"] }}
serde_json = "1.0.150"
sha2 = "0.10.9"
"#,
            manifest.display()
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        format!(
            r#"pub mod distribution {{
    pub use ultragoal::distribution::*;
}}
pub mod plugin_product {{
    pub mod distribution_adapter {{
        pub use ultragoal::plugin_product::distribution_adapter::*;
    }}
    pub mod lifecycle {{
        pub use ultragoal::plugin_product::lifecycle::*;
    }}
}}
#[path = "{}"]
pub mod host_lifecycle;
"#,
            manifest
                .join("src/plugin_product/host_lifecycle/mod.rs")
                .display()
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("src/bin/forge.rs"),
        r#"use host_lifecycle_privacy_probe::host_lifecycle::{
    capture_host_observations, HostLifecycleSession, HostObservationExpectations,
    HostObservationFrame, HostObservationTransactionRequest, HostSurfaceReader,
};

fn require_clone<T: Clone>() {}
fn require_deserializable<T: for<'de> serde::Deserialize<'de>>() {}

fn try_two_step<R: HostSurfaceReader>(session: &mut HostLifecycleSession, reader: &mut R) {
    let frame = session.capture_observations(reader).unwrap();
    let _ = session.verify_observations(frame);
}

fn main() {
    let _ = capture_host_observations;
    let _: Option<HostObservationExpectations<'static>> = None;
    require_clone::<HostObservationFrame>();
    require_deserializable::<HostObservationFrame>();
    let _ = HostObservationTransactionRequest::issue;
    require_clone::<HostObservationTransactionRequest>();
    require_deserializable::<HostObservationTransactionRequest>();
}
"#,
    )
    .unwrap();
    let output =
        std::process::Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args([
                "check",
                "--offline",
                "--quiet",
                "--jobs",
                "16",
                "--bin",
                "forge",
            ])
            .env(
                "CARGO_TARGET_DIR",
                "/tmp/hul-plugin-host-lifecycle-044-rework6-compile-red",
            )
            .current_dir(&root)
            .output()
            .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "raw observation forgery compiled");
    for expected in [
        "capture_host_observations",
        "capture_observations",
        "verify_observations",
        "HostObservationExpectations",
        "HostObservationFrame",
        "HostObservationTransactionRequest",
        "issue",
        "Clone",
        "Deserialize",
    ] {
        assert!(
            stderr.contains(expected),
            "unexpected compile failure missing {expected}: {stderr}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
