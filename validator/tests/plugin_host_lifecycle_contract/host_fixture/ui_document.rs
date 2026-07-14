pub fn ui_document(binding: &JourneyBinding) -> Vec<u8> {
    let package = binding.package();
    serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.plugins-ui-observation.v1",
        "context_id":package.source().context_id(),
        "candidate_id":package.source().candidate_id(),
        "home_id":binding.home_id(), "project_id":binding.project_id(),
        "host_id":binding.host_id(), "capability_sha256":binding.capability_sha256(),
        "binding_sha256":binding.binding_sha256(),
        "entries":[{
            "plugin_id":package.source().plugin_id(), "version":package.source().version(),
            "package_sha256":package.archive_sha256(),
            "installed_tree_sha256":package.tree_sha256(), "visible":true
        }]
    }))
    .unwrap()
}

pub fn installed(authority: &PackageAuthority, generation: u64) -> LifecycleState {
    LifecycleState {
        installed: Some(authority.clone()),
        cache: Some(authority.clone()),
        generation,
        recovery_required: false,
    }
}

pub fn request(
    intent: LifecycleIntent,
    target: Option<PackageAuthority>,
    prior: Option<LifecycleState>,
    current: Option<&PackageAuthority>,
    write: bool,
    downgrade: bool,
) -> LifecycleRequest {
    LifecycleRequest {
        intent,
        target,
        prior_authority: prior,
        authorization: LifecycleAuthorization {
            allow_host_write: write,
            allow_downgrade: downgrade,
            expected_installed_sha256: current.map(|row| row.package_sha256.clone()),
        },
    }
}

pub fn lifecycle(state: &LifecycleState, request: LifecycleRequest) -> LifecyclePlan {
    plan(state, &request).unwrap()
}

pub fn handoff_external_effect(session: &mut HostLifecycleSession) -> Option<HostCommandPlan> {
    if matches!(
        session.plan().intent,
        LifecycleIntent::RepeatUse | LifecycleIntent::IdempotentReinstall
    ) {
        return None;
    }
    let request = session.take_external_effect_request().unwrap();
    let prepared = session.consume_external_effect_request(request).unwrap();
    Some(prepared.into_plan().unwrap())
}

fn runtime_observation(
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> RuntimeObservation {
    let plan = RuntimeProbePlan::new(
        binding.clone(),
        host,
        &std::env::current_exe().unwrap(),
        vec![
            "--exact".into(),
            "host_fixture::runtime_probe_child".into(),
            "--nocapture".into(),
        ],
        Duration::from_secs(10),
    )
    .unwrap();
    execute_runtime_probe(&plan).unwrap()
}

#[test]
fn runtime_probe_child() {
    if std::env::var_os("HUL_BINDING_SHA256").is_none() {
        return;
    }
    let value = json!({
        "schema":"harness-ultragoal.runtime-probe.v1",
        "context_id":env("HUL_CONTEXT_ID"), "candidate_id":env("HUL_CANDIDATE_ID"),
        "plugin_id":env("HUL_PLUGIN_ID"), "version":env("HUL_VERSION"),
        "package_sha256":env("HUL_PACKAGE_SHA256"),
        "installed_tree_sha256":env("HUL_TREE_SHA256"),
        "home_id":env("HUL_HOME_ID"), "project_id":env("HUL_PROJECT_ID"),
        "host_id":env("HUL_HOST_ID"), "capability_sha256":env("HUL_CAPABILITY_SHA256"),
        "binding_sha256":env("HUL_BINDING_SHA256"),
        "executable_sha256":env("HUL_EXECUTABLE_SHA256"),
        "session_nonce":env("HUL_SESSION_NONCE")
    });
    println!("HUL_RUNTIME_OBSERVATION={value}");
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap()
}

fn tree(root: &Path) -> Vec<(String, &'static str, u32, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<(String, &'static str, u32, Vec<u8>)>) {
        let mut entries = fs::read_dir(path)
            .unwrap()
            .map(|row| row.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            let metadata = fs::symlink_metadata(&path).unwrap();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let mode = permission_mode(&metadata);
            if metadata.is_dir() {
                rows.push((relative, "directory", mode, Vec::new()));
                visit(root, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.push((
                    relative,
                    "symlink",
                    mode,
                    fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else {
                rows.push((relative, "file", mode, fs::read(&path).unwrap()));
            }
        }
    }
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}

#[cfg(unix)]
fn permission_mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode()
}

#[cfg(not(unix))]
fn permission_mode(_metadata: &fs::Metadata) -> u32 {
    0
}
