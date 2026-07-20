use super::state_open::prospective_state_root;
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "hul-fit-host-state-target-confinement-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self {
            root: fs::canonicalize(root).unwrap(),
        }
    }

    fn state_base(&self, home: &Path) -> PathBuf {
        let state = home.join(".codex/state");
        fs::create_dir_all(&state).unwrap();
        for path in [home, &home.join(".codex"), &state] {
            set_owner_only(path);
        }
        state
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn bootstrap_refuses_a_prospective_state_root_inside_the_target_without_writing() {
    let fixture = Fixture::new("state-inside-target");
    let target = fixture.root.join("target");
    fs::create_dir(&target).unwrap();
    let home = target.join("home");
    let state = fixture.state_base(&home);
    let expected = prospective_state_root(&state).unwrap();

    assert!(matches!(
        HostState::open(&home, &target),
        Err(HostFailure::Invalid)
    ));
    assert!(!expected.exists());
}

#[test]
fn bootstrap_refuses_a_target_inside_the_prospective_state_root_without_writing() {
    let fixture = Fixture::new("target-inside-state");
    let home = fixture.root.join("home");
    let state = fixture.state_base(&home);
    let harness = state.join("harness-ultragoal");
    fs::create_dir(&harness).unwrap();
    set_owner_only(&harness);
    let expected = harness.join("repository-fit");

    assert!(matches!(
        HostState::open(&home, &harness),
        Err(HostFailure::Invalid)
    ));
    assert!(!expected.exists());
}

fn set_owner_only(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}
