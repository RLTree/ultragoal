pub fn execute_authorized(
    plan: &HostCommandPlan,
    authorization: &HostAuthorization,
    executor: &mut impl HostExecutor,
) -> Result<HostExecutionSnapshot, DistributionError> {
    if authorization.plan_sha256 != plan.plan_sha256
        || authorization.context_id != plan.package.source().context_id()
        || authorization.candidate_id != plan.package.source().candidate_id()
    {
        return Err(error(DistributionErrorId::EffectFailed));
    }
    let mut output_sha256 = Vec::with_capacity(plan.commands.len());
    for row in &plan.commands {
        let output = executor
            .execute(&row.program, &row.argv)
            .map_err(|_| error(DistributionErrorId::EffectFailed))?;
        if output.stdout.len() > OUTPUT_LIMIT
            || output.stderr.len() > OUTPUT_LIMIT
            || output.exit_code != 0
        {
            return Err(error(DistributionErrorId::EffectFailed));
        }
        let mut combined = output.stdout;
        combined.extend_from_slice(&output.stderr);
        output_sha256.push(sha256(&combined));
    }
    Ok(HostExecutionSnapshot {
        context_id: authorization.context_id.clone(),
        candidate_id: authorization.candidate_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        command_count: plan.commands.len(),
        output_sha256,
    })
}

fn command(argv: &[&str]) -> HostCommand {
    HostCommand {
        program: "codex".to_owned(),
        argv: argv.iter().map(|row| (*row).to_owned()).collect(),
    }
}

fn bound_plan(
    package: &PackageIdentity,
    commands: Vec<HostCommand>,
) -> Result<HostCommandPlan, DistributionError> {
    package.validate()?;
    #[derive(Serialize)]
    struct Binding<'a> {
        schema: &'static str,
        package: &'a PackageIdentity,
        commands: &'a [HostCommand],
    }
    let binding = Binding {
        schema: "harness-ultragoal.host-command-plan.v1",
        package,
        commands: &commands,
    };
    let plan_sha256 = serde_json::to_vec(&binding)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    Ok(HostCommandPlan {
        package: package.clone(),
        commands,
        plan_sha256,
    })
}

fn validate_name(value: &str) -> Result<(), DistributionError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(())
}

fn validate_path_argument(value: &str) -> Result<(), DistributionError> {
    use std::path::Component;
    let mut components = std::path::Path::new(value).components();
    let shape = matches!(components.next(), Some(Component::RootDir))
        && components.all(|row| matches!(row, Component::Normal(_)));
    if value.is_empty()
        || value.len() > 4096
        || value
            .bytes()
            .any(|byte| byte == 0 || byte.is_ascii_control())
        || !shape
    {
        return Err(error(DistributionErrorId::InvalidPath));
    }
    Ok(())
}
