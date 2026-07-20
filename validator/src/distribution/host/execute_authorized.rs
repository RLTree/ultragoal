pub fn execute_authorized(
    plan: &HostCommandPlan,
    authorization: &mut HostAuthorization,
    executor: &mut impl HostExecutor,
) -> Result<HostExecutionSnapshot, DistributionError> {
    execute_authorized_report(plan, authorization, executor)
        .map_err(|_| error(DistributionErrorId::EffectFailed))
}

pub fn execute_authorized_report(
    plan: &HostCommandPlan,
    authorization: &mut HostAuthorization,
    executor: &mut impl HostExecutor,
) -> Result<HostExecutionSnapshot, HostCommandFailure> {
    if authorization.consumed {
        return Err(HostCommandFailure {
            failed_index: 0,
            completed_count: 0,
            exit: HostCommandExit::ReplayRejected,
            retry_allowed: false,
        });
    }
    if authorization.plan_sha256 != plan.plan_sha256
        || authorization.context_id != plan.package.source().context_id()
        || authorization.candidate_id != plan.package.source().candidate_id()
    {
        return Err(HostCommandFailure {
            failed_index: 0,
            completed_count: 0,
            exit: HostCommandExit::EffectUnavailable,
            retry_allowed: false,
        });
    }
    authorization.consumed = true;
    let mut output_sha256 = Vec::with_capacity(plan.commands.len());
    for (failed_index, row) in plan.commands.iter().enumerate() {
        let output = match executor.execute_with_policy(row) {
            Ok(output) => output,
            Err(error) => {
                let exit = match error {
                    HostExecutorError::BackendFailure => HostCommandExit::BackendFailure,
                    HostExecutorError::TimedOut => HostCommandExit::TimedOut,
                    HostExecutorError::Interrupted => HostCommandExit::Interrupted,
                    HostExecutorError::InvalidPolicy => HostCommandExit::EffectUnavailable,
                };
                return Err(HostCommandFailure {
                    failed_index,
                    completed_count: output_sha256.len(),
                    exit,
                    retry_allowed: false,
                });
            }
        };
        if output.stdout.len() > OUTPUT_LIMIT || output.stderr.len() > OUTPUT_LIMIT {
            return Err(HostCommandFailure {
                failed_index,
                completed_count: output_sha256.len(),
                exit: HostCommandExit::OutputLimit,
                retry_allowed: false,
            });
        }
        if output.exit_code != 0 {
            return Err(HostCommandFailure {
                failed_index,
                completed_count: output_sha256.len(),
                exit: HostCommandExit::NonZero,
                retry_allowed: false,
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distribution::{PackageIdentity, SourceIdentity};
    use std::collections::VecDeque;

    const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CANDIDATE: &str =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const CATALOG: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn package() -> PackageIdentity {
        PackageIdentity::new(
            SourceIdentity::new(
                CONTEXT.into(),
                CANDIDATE.into(),
                "harness-ultragoal".into(),
                "0.0.12".into(),
                CATALOG.into(),
                CATALOG.into(),
            )
            .expect("source identity"),
            CATALOG.into(),
            CATALOG.into(),
        )
        .expect("package identity")
    }

    #[derive(Default)]
    struct ScriptedExecutor {
        steps: VecDeque<Result<CommandOutput, HostExecutorError>>,
        seen: Vec<(String, Vec<String>, Vec<(String, String)>, u64, u8)>,
    }

    impl HostExecutor for ScriptedExecutor {
        fn execute_with_policy(
            &mut self,
            command: &HostCommand,
        ) -> Result<CommandOutput, HostExecutorError> {
            self.seen.push((
                command.program().into(),
                command.argv().to_vec(),
                command.environment().to_vec(),
                command.timeout_ms(),
                command.max_attempts(),
            ));
            self.steps.pop_front().expect("scripted step")
        }
    }

    fn authorization(plan: &HostCommandPlan) -> HostAuthorization {
        HostAuthorization::new(CONTEXT.into(), CANDIDATE.into(), plan.plan_sha256().into())
            .expect("authorization")
    }

    fn ok() -> Result<CommandOutput, HostExecutorError> {
        Ok(CommandOutput {
            exit_code: 0,
            stdout: b"ok".to_vec(),
            stderr: Vec::new(),
        })
    }

    #[test]
    fn policy_boundary_reports_partial_timeout_and_rejects_replay() {
        let plan = HostCommandPlan::repository_install(&package(), "/tmp/repository", "local-repo")
            .expect("plan");
        let mut authorization = authorization(&plan);
        let mut executor = ScriptedExecutor {
            steps: VecDeque::from([ok(), Err(HostExecutorError::TimedOut)]),
            ..Default::default()
        };
        let failure = execute_authorized_report(&plan, &mut authorization, &mut executor)
            .expect_err("timeout");
        assert_eq!(failure.failed_index(), 1);
        assert_eq!(failure.completed_count(), 1);
        assert_eq!(failure.exit(), HostCommandExit::TimedOut);
        assert!(!failure.retry_allowed());
        assert_eq!(executor.seen[0].0, "codex");
        assert_eq!(executor.seen[0].2, Vec::<(String, String)>::new());
        assert_eq!(executor.seen[0].3, 30_000);
        assert_eq!(executor.seen[0].4, 1);
        let replay = execute_authorized_report(&plan, &mut authorization, &mut executor)
            .expect_err("replay");
        assert_eq!(replay.exit(), HostCommandExit::ReplayRejected);
        assert_eq!(executor.seen.len(), 2);
    }

    #[test]
    fn policy_boundary_distinguishes_backend_nonzero_and_output_limit() {
        for (step, expected) in [
            (
                Err(HostExecutorError::BackendFailure),
                HostCommandExit::BackendFailure,
            ),
            (
                Err(HostExecutorError::Interrupted),
                HostCommandExit::Interrupted,
            ),
            (
                Err(HostExecutorError::InvalidPolicy),
                HostCommandExit::EffectUnavailable,
            ),
            (ok_with_exit(7), HostCommandExit::NonZero),
            (ok_with_output_limit(), HostCommandExit::OutputLimit),
        ] {
            let plan = HostCommandPlan::personal_install(&package(), "local-repo").expect("plan");
            let mut authorization = authorization(&plan);
            let mut executor = ScriptedExecutor {
                steps: VecDeque::from([step]),
                ..Default::default()
            };
            let failure = execute_authorized_report(&plan, &mut authorization, &mut executor)
                .expect_err("failure");
            assert_eq!(failure.failed_index(), 0);
            assert_eq!(failure.completed_count(), 0);
            assert_eq!(failure.exit(), expected);
            assert!(!failure.retry_allowed());
        }
    }

    fn ok_with_exit(exit_code: i32) -> Result<CommandOutput, HostExecutorError> {
        Ok(CommandOutput {
            exit_code,
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn ok_with_output_limit() -> Result<CommandOutput, HostExecutorError> {
        Ok(CommandOutput {
            exit_code: 0,
            stdout: vec![0; OUTPUT_LIMIT + 1],
            stderr: Vec::new(),
        })
    }
}
