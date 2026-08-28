use super::{HostEffectExecutorErrorId, recovery, session, spawn};
use crate::distribution::HostCommand;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{OpenOptionsExt, symlink};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT_PROCESS_TEST: AtomicU64 = AtomicU64::new(0);

#[test]
fn panic_boundary_requires_explicit_child_scope_finalization() {
    let cwd = File::open(".").expect("test cwd");
    let command = HostCommand::from_untrusted_record(
        "codex".to_owned(),
        vec!["--version".to_owned()],
        Vec::new(),
        1_000,
        1,
    );
    let spawned = spawn::spawn(std::path::Path::new("/bin/sh"), &command, cwd.as_raw_fd())
        .expect("suspended child");
    let session = session::ChildSession::new(spawned);
    let panic_result = catch_unwind(AssertUnwindSafe(|| panic!("panic control")));
    assert!(panic_result.is_err());

    let failure = session.finalize_failure(HostEffectExecutorErrorId::ProcessFailed);
    assert!(failure.started);
}

#[test]
fn spawn_boundary_codex_home_symlink_race_cannot_write_outside_retained_authority() {
    let root = process_test_root("codex-home-race");
    let home = root.join("home");
    let codex_home = home.join(".codex");
    let held_codex_home = home.join(".codex-held");
    let outside = root.join("outside");
    fs::create_dir_all(&codex_home).unwrap();
    fs::create_dir(&outside).unwrap();
    let lease = lock_home(&home, false).expect("exclusive lease");
    let cwd = File::open(&home).unwrap();
    let command = HostCommand::from_untrusted_record(
        "codex".to_owned(),
        vec![
            "-c".to_owned(),
            "printf escaped > \"$CODEX_HOME/escaped\"".to_owned(),
        ],
        vec![
            ("HOME".to_owned(), home.display().to_string()),
            ("CODEX_HOME".to_owned(), codex_home.display().to_string()),
        ],
        5_000,
        1,
    );
    let policy = crate::distribution::host_effect::executor::HostEffectExecutionPolicy::strict_personal_codex_home(
        5_000,
        &home,
    )
    .unwrap();
    let spawned = spawn::spawn_personal(
        Path::new("/bin/sh"),
        &command,
        cwd.as_raw_fd(),
        lease.as_raw_fd(),
        &codex_home,
    )
    .expect("suspended confined child");

    fs::rename(&codex_home, &held_codex_home).unwrap();
    symlink(&outside, &codex_home).unwrap();
    let cancellation =
        crate::distribution::host_effect::executor::HostEffectCancellation::default();
    let result = session::ChildSession::new(spawned).run_guarded(&policy, &cancellation);

    assert!(result.is_err(), "redirected write unexpectedly succeeded");
    assert!(!outside.join("escaped").exists());
    fs::remove_file(&codex_home).unwrap();
    fs::rename(&held_codex_home, &codex_home).unwrap();
    drop(lease);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn parent_death_inherited_lease_blocks_competing_apply_until_child_group_is_terminal() {
    if std::env::var_os("HUL_P1A_LEASE_PARENT").is_some() {
        lease_parent_helper();
        return;
    }
    let root = process_test_root("parent-death-lease");
    let home = root.join("home");
    fs::create_dir_all(home.join(".codex")).unwrap();
    let executable = std::env::current_exe().unwrap();
    let mut parent = Command::new(executable)
        .arg("parent_death_inherited_lease_blocks_competing_apply_until_child_group_is_terminal")
        .arg("--nocapture")
        .env("HUL_P1A_LEASE_PARENT", "1")
        .env("HUL_P1A_LEASE_HOME", &home)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let stdout = parent.stdout.take().unwrap();
    let child_pid = BufReader::new(stdout)
        .lines()
        .map(Result::unwrap)
        .find_map(|line| {
            line.strip_prefix("HUL_CHILD=")
                .and_then(|value| value.parse::<libc::pid_t>().ok())
        })
        .expect("helper child pid");
    assert_eq!(
        unsafe { libc::kill(parent.id() as libc::pid_t, libc::SIGKILL) },
        0
    );
    let _ = parent.wait().unwrap();

    assert!(
        lock_home(&home, true).is_err(),
        "second apply entered early"
    );
    unsafe {
        libc::kill(-child_pid, libc::SIGKILL);
        libc::kill(child_pid, libc::SIGKILL);
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    while recovery::group_exists(child_pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!recovery::group_exists(child_pid));
    let next = lock_home(&home, true).expect("lease released after child terminal");
    drop(next);
    fs::remove_dir_all(root).unwrap();
}

fn lease_parent_helper() {
    let home = PathBuf::from(std::env::var_os("HUL_P1A_LEASE_HOME").unwrap());
    let lease = lock_home(&home, false).expect("helper lease");
    let cwd = File::open(&home).unwrap();
    let codex_home = home.join(".codex");
    let command = HostCommand::from_untrusted_record(
        "codex".to_owned(),
        vec![
            "-e".to_owned(),
            "select(undef, undef, undef, 60);".to_owned(),
        ],
        vec![
            ("HOME".to_owned(), home.display().to_string()),
            ("CODEX_HOME".to_owned(), codex_home.display().to_string()),
        ],
        60_000,
        1,
    );
    let spawned = spawn::spawn_personal(
        Path::new("/usr/bin/perl"),
        &command,
        cwd.as_raw_fd(),
        lease.as_raw_fd(),
        &codex_home,
    )
    .expect("helper child");
    assert_eq!(unsafe { libc::kill(-spawned.pid, libc::SIGCONT) }, 0);
    println!("HUL_CHILD={}", spawned.pid);
    std::io::stdout().flush().unwrap();
    std::mem::forget(spawned);
    std::thread::sleep(Duration::from_secs(60));
    drop(lease);
}

fn lock_home(home: &Path, nonblocking: bool) -> Result<File, ()> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY);
    let file = options.open(home).map_err(|_| ())?;
    let operation = libc::LOCK_EX | if nonblocking { libc::LOCK_NB } else { 0 };
    if unsafe { libc::flock(file.as_raw_fd(), operation) } != 0 {
        return Err(());
    }
    Ok(file)
}

fn process_test_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "hul-darwin-{label}-{}-{}",
        std::process::id(),
        NEXT_PROCESS_TEST.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    root.canonicalize().unwrap()
}
