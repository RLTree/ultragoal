const OUTPUT_LIMIT: usize = 1024 * 1024;
const EXECUTABLE_LIMIT: usize = 256 * 1024 * 1024;
const MARKER: &[u8] = b"HUL_RUNTIME_OBSERVATION=";
static NONCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct RuntimeProbePlan {
    binding: JourneyBinding,
    program: PathBuf,
    argv: Vec<String>,
    executable_sha256: String,
    session_nonce: String,
    timeout: Duration,
}

impl RuntimeProbePlan {
    pub fn new(
        binding: JourneyBinding,
        host: &HostCapabilityDeclaration,
        program: &Path,
        argv: Vec<String>,
        timeout: Duration,
    ) -> Result<Self, DistributionError> {
        if host.state(Capability::Runtime) != HostCapabilityState::Supported
            || !program.is_absolute()
            || argv.len() > 32
            || argv.iter().any(|row| {
                row.len() > 4096 || row.bytes().any(|byte| byte == 0 || byte.is_ascii_control())
            })
            || timeout.is_zero()
            || timeout > Duration::from_secs(30)
        {
            return Err(error(DistributionErrorId::CapabilityMismatch));
        }
        let executable_sha256 = executable_digest(program)?;
        let session_nonce = sha256(
            format!(
                "{}\0{}\0{}",
                binding.binding_sha256(),
                std::process::id(),
                NONCE.fetch_add(1, Ordering::Relaxed)
            )
            .as_bytes(),
        );
        Ok(Self {
            binding,
            program: program.into(),
            argv,
            executable_sha256,
            session_nonce,
            timeout,
        })
    }

    pub fn execute_bound(self) -> Result<(RuntimeObservation, SurfaceIdentity), DistributionError> {
        let observation = execute_runtime_probe(&self)?;
        let output_sha256 = observation
            .output_sha256()
            .ok_or_else(|| error(DistributionErrorId::ProvenanceMismatch))?;
        let surface = SurfaceIdentity::new(
            self.binding.package().clone(),
            IdentitySurface::Runtime,
            output_sha256.into(),
            None,
        )?
        .bind_journey(&self.binding)?;
        Ok((observation, surface))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeEnvelope {
    schema: String,
    context_id: String,
    candidate_id: String,
    plugin_id: String,
    version: String,
    package_sha256: String,
    installed_tree_sha256: String,
    home_id: String,
    project_id: String,
    host_id: String,
    capability_sha256: String,
    binding_sha256: String,
    executable_sha256: String,
    session_nonce: String,
}

pub fn execute_runtime_probe(
    plan: &RuntimeProbePlan,
) -> Result<RuntimeObservation, DistributionError> {
    if executable_digest(&plan.program)? != plan.executable_sha256 {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    let source = plan.binding.package().source();
    let mut command = Command::new(&plan.program);
    command
        .args(&plan.argv)
        .env_clear()
        .env("HUL_CONTEXT_ID", source.context_id())
        .env("HUL_CANDIDATE_ID", source.candidate_id())
        .env("HUL_PLUGIN_ID", source.plugin_id())
        .env("HUL_VERSION", source.version())
        .env(
            "HUL_PACKAGE_SHA256",
            plan.binding.package().archive_sha256(),
        )
        .env("HUL_TREE_SHA256", plan.binding.package().tree_sha256())
        .env("HUL_HOME_ID", plan.binding.home_id())
        .env("HUL_PROJECT_ID", plan.binding.project_id())
        .env("HUL_HOST_ID", plan.binding.host_id())
        .env("HUL_CAPABILITY_SHA256", plan.binding.capability_sha256())
        .env("HUL_BINDING_SHA256", plan.binding.binding_sha256())
        .env("HUL_EXECUTABLE_SHA256", &plan.executable_sha256)
        .env("HUL_SESSION_NONCE", &plan.session_nonce)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    let deadline = Instant::now() + plan.timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error(DistributionErrorId::EffectFailed));
            }
            Err(_) => return Err(error(DistributionErrorId::EffectFailed)),
        }
    };
    let stdout = read_output(child.stdout.take())?;
    let stderr = read_output(child.stderr.take())?;
    if !status.success() {
        return Err(error(DistributionErrorId::EffectFailed));
    }
    let marker = unique_marker(&stdout)?;
    let envelope: ProbeEnvelope = json::parse(marker, 64 * 1024)?;
    validate_envelope(&envelope, plan)?;
    if executable_digest(&plan.program)? != plan.executable_sha256 {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    let mut output = stdout;
    output.extend_from_slice(&stderr);
    Ok(RuntimeObservation::executed(
        &plan.binding,
        plan.executable_sha256.clone(),
        sha256(&output),
    ))
}

fn validate_envelope(
    row: &ProbeEnvelope,
    plan: &RuntimeProbePlan,
) -> Result<(), DistributionError> {
    let source = plan.binding.package().source();
    if row.schema != "harness-ultragoal.runtime-probe.v1"
        || row.context_id != source.context_id()
        || row.candidate_id != source.candidate_id()
        || row.plugin_id != source.plugin_id()
        || row.version != source.version()
        || row.package_sha256 != plan.binding.package().archive_sha256()
        || row.installed_tree_sha256 != plan.binding.package().tree_sha256()
        || row.home_id != plan.binding.home_id()
        || row.project_id != plan.binding.project_id()
        || row.host_id != plan.binding.host_id()
        || row.capability_sha256 != plan.binding.capability_sha256()
        || row.binding_sha256 != plan.binding.binding_sha256()
        || row.executable_sha256 != plan.executable_sha256
        || row.session_nonce != plan.session_nonce
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(())
}
