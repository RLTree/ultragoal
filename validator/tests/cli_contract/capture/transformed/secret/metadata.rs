#[cfg(target_os = "macos")]
mod macos {
    use super::super::capture::{CommandSpec, PublicArg, PublicArtifact, PublicEnv, SecretArg};
    use super::super::fixture::RepoFixture;
    use super::super::secret_capture_fixture::assert_serialized_absent;
    use crate::context::{BuildRequest, LiveContext};
    use std::path::Path;

    fn context(fixture: &RepoFixture) -> LiveContext {
        LiveContext::build(
            BuildRequest::new(fixture.root())
                .expect_repository_root(fixture.root())
                .expect_worktree_root(fixture.root())
                .bind_secret_source("ARG_SECRET", "v1")
                .probe_tool("sandbox-exec")
                .probe_tool("true"),
        )
        .unwrap()
    }

    #[test]
    fn caller_declared_transforms_are_withheld_from_every_invocation_surface() {
        let fixture = RepoFixture::new("transformed-secret-input-metadata");
        std::fs::create_dir(fixture.root().join("123456")).unwrap();
        fixture.write_file("out/1e240.bin", b"prefix-1e240-suffix");
        let context = context(&fixture);
        let secret = "+123456";
        let transforms = [
            "123456",
            "1e240",
            "1E240",
            "prefix-1e240-suffix",
            "KzEyMzQ1Ng==",
            "\\x2b\\x31\\x32\\x33\\x34\\x35\\x36",
        ];

        let mut public = CommandSpec::catalog_read("transformed-metadata-public", "true")
            .cwd("123456")
            .public_artifact(PublicArtifact::new("out/1e240.bin"));
        for (index, value) in transforms.iter().enumerate() {
            public = public
                .public_arg(PublicArg::new(*value))
                .public_environment(PublicEnv::new(format!("PUBLIC_{index}"), *value));
        }
        let public = public.run(&context).unwrap();
        let public_json = public.to_canonical_json().unwrap();
        let public_value: serde_json::Value = serde_json::from_slice(&public_json).unwrap();
        assert_eq!(public_value["invocation_metadata_disposition"], "public");
        assert!(public_value.get("cwd_path_hex").is_some());
        assert!(
            public_value["enforcement"]
                .get("worktree_read_scope")
                .is_some()
        );
        assert!(
            public_value["arguments"]
                .as_array()
                .unwrap()
                .iter()
                .all(|record| record.get("value_hex").is_some())
        );
        assert!(
            public_value["environment"]
                .as_array()
                .unwrap()
                .iter()
                .all(|record| record.get("name_hex").is_some()
                    && record.get("value_hex").is_some())
        );
        assert_eq!(
            public.artifacts()[0].relative_path().unwrap(),
            Path::new("out/1e240.bin")
        );

        let mut sensitive = CommandSpec::catalog_read("transformed-metadata-secret", "true")
            .cwd("123456")
            .secret_arg(SecretArg::new(secret, "ARG_SECRET"))
            .public_artifact(PublicArtifact::new("out/1e240.bin"));
        for (index, value) in transforms.iter().enumerate() {
            sensitive = sensitive
                .public_arg(PublicArg::new(*value))
                .public_environment(PublicEnv::new(format!("PUBLIC_{index}"), *value));
        }
        let sensitive = sensitive.run(&context).unwrap();
        let json = sensitive.to_canonical_json().unwrap();
        let debug = format!("{sensitive:?}");
        let mut needles = transforms
            .iter()
            .map(|value| value.as_bytes())
            .collect::<Vec<_>>();
        needles.extend([secret.as_bytes(), b"out/1e240.bin", b"ARG_SECRET"]);
        assert_serialized_absent(&json, &needles);
        assert_serialized_absent(debug.as_bytes(), &needles);
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(
            value["invocation_metadata_disposition"],
            "withheld-secret-bearing-invocation"
        );
        assert!(value.get("cwd_path_hex").is_none());
        assert!(value["enforcement"].get("worktree_read_scope").is_none());
        for collection in ["arguments", "environment"] {
            assert!(value[collection].as_array().unwrap().iter().all(|record| {
                record.as_object().unwrap().len() == 1
                    && record["channel"] == "withheld-secret-bearing-invocation"
            }));
        }
        assert_eq!(sensitive.artifacts()[0].relative_path(), None);
    }
}
