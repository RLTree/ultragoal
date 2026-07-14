use super::*;

pub(crate) const BASE: &str = "/tmp/hul-repository-fit-apply-mediation-058";
pub(crate) const SECRET: &[u8] = b"repository-fit-test-root-authority-secret-material-v1";
pub(crate) static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

pub(crate) struct Fixture {
    pub(crate) container: PathBuf,
    pub(crate) root: PathBuf,
}

impl Fixture {
    pub(crate) fn new(label: &str) -> Self {
        let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let container = PathBuf::from(BASE).join(format!(
            "{}-{}-{serial}",
            label.replace(|character: char| !character.is_ascii_alphanumeric(), "-"),
            std::process::id()
        ));
        let root = container.join("repo");
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "--quiet"]);
        Self { container, root }
    }

    pub(crate) fn context(&self) -> LiveContext {
        LiveContext::build(BuildRequest::new(&self.root)).unwrap()
    }

    pub(crate) fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    pub(crate) fn write_template(&self, target: &str) {
        let row = CANONICAL_TEMPLATES
            .iter()
            .find(|row| row.target_path == target)
            .unwrap();
        self.write(target, row.bytes);
        fs::set_permissions(
            self.root.join(target),
            fs::Permissions::from_mode(row.unix_mode),
        )
        .unwrap();
    }

    pub(crate) fn install_all(&self) {
        for row in CANONICAL_TEMPLATES {
            self.write(row.target_path, row.bytes);
            fs::set_permissions(
                self.root.join(row.target_path),
                fs::Permissions::from_mode(row.unix_mode),
            )
            .unwrap();
        }
    }

    pub(crate) fn request(&self, context: &LiveContext) -> OpaqueFitApplyRequest {
        let record = plan_target(context).unwrap();
        prepare_apply_request(
            context,
            &record.to_machine_bytes().unwrap(),
            record.plan_sha256(),
        )
        .unwrap()
        .into_request()
    }

    pub(crate) fn effects(&self, request: &OpaqueFitApplyRequest) -> LocalEffects {
        LocalEffects::open_for_test(&self.root, request.unix_modes.clone()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
}

pub(crate) fn new_authority() -> TestRepositoryFitPermitAuthority {
    TestRepositoryFitPermitAuthority::new(SECRET).unwrap()
}

pub(crate) fn expect_apply_ok<E: RepositoryFitPermitEffects>(
    result: Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>>,
) -> RepositoryFitApplyOutcome {
    match result {
        Ok(outcome) => outcome,
        Err(failure) => panic!("apply failed with {:?}", failure.error().id()),
    }
}

pub(crate) fn expect_apply_failure<E: RepositoryFitPermitEffects>(
    result: Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>>,
) -> RepositoryFitApplyFailure<E> {
    match result {
        Err(failure) => failure,
        Ok(outcome) => panic!("unexpected {} apply outcome", outcome.status()),
    }
}

pub(crate) fn nonce(label: &str) -> Vec<u8> {
    digest(label.as_bytes()).into_bytes()
}

pub(crate) fn git(root: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(arguments)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
}

pub(crate) fn catalog_ancestor_paths() -> BTreeSet<PathBuf> {
    let mut ancestors = BTreeSet::new();
    for row in CANONICAL_TEMPLATES {
        let mut parent = Path::new(row.target_path).parent();
        while let Some(path) = parent {
            if path.as_os_str().is_empty() {
                break;
            }
            ancestors.insert(path.to_path_buf());
            parent = path.parent();
        }
    }
    ancestors
}

pub(crate) fn replace_directory_preserving_children(path: &Path, displaced: &Path) {
    fs::rename(path, displaced).unwrap();
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    for entry in fs::read_dir(displaced).unwrap() {
        let entry = entry.unwrap();
        fs::rename(entry.path(), path.join(entry.file_name())).unwrap();
    }
}

pub(crate) fn replace_directory_with_fifo(path: &Path, displaced: &Path) {
    fs::rename(path, displaced).unwrap();
    let encoded = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(encoded.as_ptr(), 0o600) }, 0);
}

pub(crate) fn arm_capture_barrier(
    phase: TargetCapturePhase,
    path: &str,
    mutate: impl FnOnce() + Send + 'static,
) -> JoinHandle<()> {
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    let attacker_reached = Arc::clone(&reached);
    let attacker_resume = Arc::clone(&resume);
    let attacker = std::thread::spawn(move || {
        attacker_reached.wait();
        mutate();
        attacker_resume.wait();
    });
    target_capture_hook_for_test(phase, path, move || {
        reached.wait();
        resume.wait();
    });
    attacker
}
