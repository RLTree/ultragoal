use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct ScratchDirectory {
    base: PathBuf,
    container: PathBuf,
    cleaned: bool,
}

impl ScratchDirectory {
    pub(crate) fn new(namespace: &str, label: &str, unique: u64) -> Self {
        let base = std::env::temp_dir()
            .join("harness-ultragoal-observability-tests")
            .join(namespace);
        fs::create_dir_all(&base).expect("create confined fixture root");
        let base = fs::canonicalize(base).expect("canonical confined fixture root");
        let container = base.join(format!("{label}-{}-{unique}", std::process::id()));
        fs::create_dir(&container).expect("create unique fixture container");
        let container = fs::canonicalize(container).expect("canonical fixture container");
        assert_eq!(
            container.parent(),
            Some(base.as_path()),
            "fixture container escaped its confined temporary root"
        );
        Self {
            base,
            container,
            cleaned: false,
        }
    }

    pub(crate) fn container(&self) -> &Path {
        &self.container
    }

    pub(crate) fn teardown(mut self) {
        assert!(
            self.container.starts_with(&self.base),
            "fixture cleanup escaped its confined temporary root"
        );
        fs::remove_dir_all(&self.container).expect("explicit fixture teardown");
        assert!(
            !self.container.exists(),
            "explicit fixture teardown left its container behind"
        );
        self.cleaned = true;
    }
}

impl Drop for ScratchDirectory {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert!(
                self.cleaned,
                "fixture requires explicit checked teardown; leaked for inspection"
            );
        }
    }
}
