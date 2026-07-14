#[cfg(not(unix))]
pub(crate) fn read_descriptor(
    _file: &File,
    _maximum_bytes: usize,
) -> Result<(Vec<u8>, String), String> {
    Err("descriptor-confined capture requires Unix".to_owned())
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::{open_file_at, reset_test_file_open_attempts, test_file_open_attempts};
    use std::ffi::{CString, OsStr};
    use std::fs::{self, File};
    use std::os::unix::ffi::OsStrExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIFO: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn executable_fifo_is_rejected_before_file_open() {
        let id = NEXT_FIFO.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "ultragoal-capture-fifo-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("create FIFO fixture root");
        let fifo = root.join("program");
        let fifo_c = CString::new(fifo.as_os_str().as_bytes()).expect("FIFO path CString");
        assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o700) }, 0);
        let directory = File::open(&root).expect("open FIFO fixture root");
        reset_test_file_open_attempts();
        assert!(open_file_at(&directory, OsStr::new("program"), true).is_err());
        assert_eq!(test_file_open_attempts(), 0);
        fs::remove_dir_all(root).expect("remove FIFO fixture root");
    }
}
