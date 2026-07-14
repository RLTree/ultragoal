use super::*;

pub(crate) const BASE: &str = "/tmp/hul-repository-fit-live-journeys-067";
pub(crate) const TEST_ROOT_SECRET: &[u8] = b"repository-fit-live-journey-test-root-authority-067";
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
        git(
            &root,
            &["config", "user.email", "fit-journey@example.invalid"],
        );
        git(&root, &["config", "user.name", "Repository Fit Journey"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        git(&root, &["config", "gc.auto", "0"]);
        git(&root, &["config", "maintenance.auto", "false"]);
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
        self.write(row.target_path, row.bytes);
        fs::set_permissions(
            self.root.join(row.target_path),
            fs::Permissions::from_mode(row.unix_mode),
        )
        .unwrap();
    }

    pub(crate) fn install_all_direct(&self) {
        for row in CANONICAL_TEMPLATES {
            self.write(row.target_path, row.bytes);
            fs::set_permissions(
                self.root.join(row.target_path),
                fs::Permissions::from_mode(row.unix_mode),
            )
            .unwrap();
        }
    }

    pub(crate) fn commit_all(&self, message: &str) {
        git(&self.root, &["add", "-A"]);
        git(&self.root, &["commit", "--quiet", "-m", message]);
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
        LocalEffects::open_for_test(&self.root, request.unix_modes().clone()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
}

pub(crate) fn git(root: &Path, args: &[&str]) -> Output {
    let output = Command::new("/usr/bin/git")
        .args(args)
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    output
}

pub(crate) fn authority() -> TestRepositoryFitPermitAuthority {
    TestRepositoryFitPermitAuthority::new(TEST_ROOT_SECRET).unwrap()
}

pub(crate) fn nonce(label: &str) -> Vec<u8> {
    digest(label.as_bytes()).into_bytes()
}

pub(crate) fn issue<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    effects: E,
    label: &str,
) -> (
    crate::repository_fit::product_adapter::root_permit::RepositoryFitApplyPermit,
    crate::repository_fit::product_adapter::root_permit::RepositoryFitMutationLease<E>,
) {
    authority()
        .issue(context, request, effects, 10, 20, &nonce(label))
        .unwrap()
}

pub(crate) fn apply_ok<E: RepositoryFitPermitEffects>(
    result: Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>>,
) -> RepositoryFitApplyOutcome {
    match result {
        Ok(outcome) => outcome,
        Err(failure) => panic!("apply failed with {:?}", failure.error().id()),
    }
}

pub(crate) fn apply_failure<E: RepositoryFitPermitEffects>(
    result: Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>>,
) -> RepositoryFitApplyFailure<E> {
    match result {
        Err(failure) => failure,
        Ok(outcome) => panic!("unexpected {} outcome", outcome.status()),
    }
}

pub(crate) fn apply_once(
    fixture: &Fixture,
    context: &LiveContext,
    request: OpaqueFitApplyRequest,
    label: &str,
) -> RepositoryFitApplyOutcome {
    let (permit, lease) = issue(context, &request, fixture.effects(&request), label);
    apply_ok(apply_with_root_permit(
        context,
        request,
        Some(permit),
        Some(lease),
        10,
    ))
}

pub(crate) fn assert_zero_write<T>(fixture: &Fixture, operation: impl FnOnce() -> T) -> T {
    let before_tree = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let result = operation();
    assert_eq!(snapshot(&fixture.root), before_tree);
    assert_eq!(git_status(&fixture.root), before_status);
    result
}
