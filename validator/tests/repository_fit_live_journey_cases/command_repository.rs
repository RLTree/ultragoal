use super::*;

pub(crate) struct CommandRepository {
    pub(crate) container: PathBuf,
    pub(crate) root: PathBuf,
}

impl CommandRepository {
    pub(crate) fn new() -> Self {
        let container = PathBuf::from(BASE).join(format!(
            "command-zero-write-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let root = container.join("repo");
        fs::create_dir_all(root.join("nested/empty")).unwrap();
        let root = fs::canonicalize(root).unwrap();
        git(&root, &["init", "--quiet"]);
        git(
            &root,
            &["config", "user.email", "fit-command@example.invalid"],
        );
        git(&root, &["config", "user.name", "Repository Fit Command"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        git(&root, &["config", "gc.auto", "0"]);
        git(&root, &["config", "maintenance.auto", "false"]);
        fs::write(root.join("tracked.txt"), b"tracked baseline\n").unwrap();
        fs::write(root.join("nested/linked.txt"), b"linked bytes\n").unwrap();
        std::os::unix::fs::symlink("linked.txt", root.join("nested/linked-symlink")).unwrap();
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "--quiet", "-m", "command baseline"]);
        fs::write(root.join("tracked.txt"), b"tracked dirty user edit\n").unwrap();
        fs::write(root.join("private-canary.txt"), PRIVATE_CANARY).unwrap();
        Self { container, root }
    }

    pub(crate) fn run(&self, args: &[String]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ultragoal"))
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.root)
            .arg("--root")
            .arg(&self.root)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for CommandRepository {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
}
