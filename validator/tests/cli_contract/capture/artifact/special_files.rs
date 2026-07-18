#[cfg(unix)]
mod unix {
    use super::super::capture::{
        PublicArtifact, capture_public_for_test, reset_test_descriptor_bytes_read,
        reset_test_file_open_attempts, set_test_artifact_pause_ms, set_test_preopen_pause_ms,
        test_artifact_is_paused, test_descriptor_bytes_read, test_file_open_attempts,
        test_preopen_is_paused,
    };
    use super::super::fixture::RepoFixture;
    use std::ffi::CString;
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::net::UnixListener;
    use std::time::{Duration, Instant};

    fn assert_rejected_without_open(fixture: &RepoFixture, relative: &str) {
        reset_test_descriptor_bytes_read();
        reset_test_file_open_attempts();
        let error =
            capture_public_for_test(&fixture.context(), vec![PublicArtifact::new(relative)])
                .unwrap_err();
        assert!(error.contains("regular-file open failed"));
        assert_eq!(test_file_open_attempts(), 0);
        assert_eq!(test_descriptor_bytes_read(), 0);
    }

    #[test]
    fn fifo_artifact_is_rejected_before_file_open() {
        let fixture = RepoFixture::new("artifact-special-files");
        std::fs::create_dir_all(fixture.root().join("out")).unwrap();
        let fifo = fixture.root().join("out/fifo");
        let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
        assert_rejected_without_open(&fixture, "out/fifo");
    }

    #[test]
    fn socket_artifact_is_rejected_before_file_open() {
        let fixture = RepoFixture::new("artifact-socket");
        std::fs::create_dir_all(fixture.root().join("out")).unwrap();
        let socket = fixture.root().join("out/socket");
        let _listener = UnixListener::bind(&socket).expect("bind real socket fixture");
        assert_rejected_without_open(&fixture, "out/socket");
    }

    #[test]
    fn regular_artifact_replaced_by_fifo_is_rejected_without_opening_the_fifo() {
        let fixture = RepoFixture::new("artifact-regular-to-fifo");
        fixture.write_file("out/result", b"original");
        let context = fixture.context();
        let root = fixture.root().to_path_buf();
        reset_test_descriptor_bytes_read();
        reset_test_file_open_attempts();
        set_test_artifact_pause_ms(100);
        let swapper = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !test_artifact_is_paused() {
                assert!(
                    Instant::now() < deadline,
                    "artifact capture did not reach pause"
                );
                std::thread::sleep(Duration::from_millis(1));
            }
            fs::rename(root.join("out/result"), root.join("out/original")).unwrap();
            let fifo = root.join("out/result");
            let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
        });

        let error =
            capture_public_for_test(&context, vec![PublicArtifact::new("out/result")]).unwrap_err();
        swapper.join().unwrap();
        assert!(
            error.contains("regular-file open failed")
                || error.contains("identity changed")
                || error.contains("context")
        );
        assert_eq!(test_file_open_attempts(), 1);
        assert_eq!(test_descriptor_bytes_read(), b"original".len() as u64);
    }

    #[test]
    fn regular_to_fifo_swap_in_the_stat_open_window_is_nonblocking_and_fail_closed() {
        let fixture = RepoFixture::new("artifact-stat-open-fifo-swap");
        fixture.write_file("out/result", b"original");
        let context = fixture.context();
        let root = fixture.root().to_path_buf();
        reset_test_descriptor_bytes_read();
        reset_test_file_open_attempts();
        set_test_preopen_pause_ms(100);
        let swapper = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !test_preopen_is_paused() {
                assert!(Instant::now() < deadline, "pre-open pause was not reached");
                std::thread::sleep(Duration::from_millis(1));
            }
            fs::rename(root.join("out/result"), root.join("out/original")).unwrap();
            let fifo = root.join("out/result");
            let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
        });

        let started = Instant::now();
        let error =
            capture_public_for_test(&context, vec![PublicArtifact::new("out/result")]).unwrap_err();
        swapper.join().unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(error.contains("identity changed") || error.contains("regular-file open failed"));
        assert_eq!(test_file_open_attempts(), 1);
        assert_eq!(test_descriptor_bytes_read(), 0);
    }
}
