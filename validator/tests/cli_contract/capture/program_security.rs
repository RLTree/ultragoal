#[cfg(target_os = "macos")]
mod macos {
    use super::super::capture::CommandSpec;
    use super::super::fixture::RepoFixture;
    use crate::context::{BuildRequest, LiveContext};
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    struct RestorePath(Option<OsString>);

    impl Drop for RestorePath {
        fn drop(&mut self) {
            if let Some(path) = self.0.take() {
                unsafe { std::env::set_var("PATH", path) };
            } else {
                unsafe { std::env::remove_var("PATH") };
            }
        }
    }

    #[test]
    fn path_shadowed_supported_program_is_rejected_before_launch() {
        let _serial = crate::serial();
        let fixture = RepoFixture::new("tampered-supported-program");
        let marker = fixture.root().join("fake-printf-ran");
        fixture.write_script(
            "fake-bin/printf",
            &format!("printf ran > '{}'", marker.display()),
        );
        fs::set_permissions(
            fixture.root().join("fake-bin/printf"),
            fs::Permissions::from_mode(0o777),
        )
        .unwrap();
        let original = std::env::var_os("PATH");
        let _restore = RestorePath(original.clone());
        let mut paths = vec![fixture.root().join("fake-bin")];
        paths.extend(std::env::split_paths(
            original.as_deref().unwrap_or_default(),
        ));
        unsafe { std::env::set_var("PATH", std::env::join_paths(paths).unwrap()) };
        let context = LiveContext::build(
            BuildRequest::new(fixture.root())
                .probe_tool("sandbox-exec")
                .probe_tool("printf"),
        )
        .unwrap();

        let error = CommandSpec::catalog_read("test-read", "printf")
            .run(&context)
            .unwrap_err();

        assert!(error.contains("not a protected native file"));
        assert!(!error.contains("ran"));
        assert!(!marker.exists());
    }
}
