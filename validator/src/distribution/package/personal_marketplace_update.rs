use crate::plugin_product::lifecycle::PriorInstalledAuthority;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PersonalMarketplaceUpdateAuthority {
    plan_sha256: String,
    prior: PriorInstalledAuthority,
    prior_tree_sha256: String,
    target_package_sha256: String,
    target_tree_sha256: String,
}

impl PersonalMarketplaceUpdateAuthority {
    pub(crate) fn new(
        plan_sha256: String,
        prior: PriorInstalledAuthority,
        prior_tree_sha256: String,
        target_package_sha256: String,
        target_tree_sha256: String,
    ) -> Result<Self, &'static str> {
        let authority = Self {
            plan_sha256,
            prior,
            prior_tree_sha256,
            target_package_sha256,
            target_tree_sha256,
        };
        authority.validate()?;
        Ok(authority)
    }

    fn validate(&self) -> Result<(), &'static str> {
        if !digest(&self.plan_sha256)
            || self.prior.validate().is_err()
            || !digest(&self.prior_tree_sha256)
            || !digest(&self.target_package_sha256)
            || !digest(&self.target_tree_sha256)
            || self.prior_tree_sha256 == self.target_tree_sha256
            || self.prior.authority().package_sha256 == self.target_package_sha256
        {
            return Err("personal marketplace update authority is invalid");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PersonalMarketplaceAuthorityMatch {
    Prior,
    Target,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PersonalMarketplaceUpdateObservation {
    pub(crate) marketplace_root_bound: bool,
    pub(crate) source_tree: PersonalMarketplaceAuthorityMatch,
    pub(crate) installed: PersonalMarketplaceAuthorityMatch,
    pub(crate) cache: PersonalMarketplaceAuthorityMatch,
    pub(crate) runtime: PersonalMarketplaceAuthorityMatch,
    pub(crate) registry: PersonalMarketplaceAuthorityMatch,
}

impl PersonalMarketplaceUpdateObservation {
    pub(crate) fn unavailable() -> Self {
        Self {
            marketplace_root_bound: false,
            source_tree: PersonalMarketplaceAuthorityMatch::Other,
            installed: PersonalMarketplaceAuthorityMatch::Other,
            cache: PersonalMarketplaceAuthorityMatch::Other,
            runtime: PersonalMarketplaceAuthorityMatch::Other,
            registry: PersonalMarketplaceAuthorityMatch::Other,
        }
    }

    fn exact_prior(self) -> bool {
        self.marketplace_root_bound
            && [
                self.source_tree,
                self.installed,
                self.cache,
                self.runtime,
                self.registry,
            ]
            .into_iter()
            .all(|value| value == PersonalMarketplaceAuthorityMatch::Prior)
    }

    fn exact_target(self) -> bool {
        self.marketplace_root_bound
            && [
                self.source_tree,
                self.installed,
                self.cache,
                self.runtime,
                self.registry,
            ]
            .into_iter()
            .all(|value| value == PersonalMarketplaceAuthorityMatch::Target)
    }

    fn target_tree_only(self) -> bool {
        self.marketplace_root_bound
            && self.source_tree == PersonalMarketplaceAuthorityMatch::Target
            && [self.installed, self.cache, self.runtime, self.registry]
                .into_iter()
                .all(|value| value == PersonalMarketplaceAuthorityMatch::Prior)
    }

    fn prior_tree_is_restored(self) -> bool {
        self.marketplace_root_bound && self.source_tree == PersonalMarketplaceAuthorityMatch::Prior
    }
}

pub(crate) trait PersonalMarketplaceUpdateEffects {
    fn observe(
        &mut self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<PersonalMarketplaceUpdateObservation, &'static str>;

    fn materialize_target(
        &mut self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<(), &'static str>;

    fn install_target(
        &mut self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<(), &'static str>;

    fn restore_prior_tree(
        &mut self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<(), &'static str>;

    fn reinstall_prior(
        &mut self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<(), &'static str>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PersonalMarketplaceUpdateStage {
    InitialObservation,
    InterruptedAfterMaterialization,
    MaterializeTarget,
    ReconcileMaterializedTarget,
    InstallTarget,
    ReconcileTarget,
    RestorePriorTree,
    ReinstallPrior,
    ReconcilePrior,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PersonalMarketplaceUpdateDisposition {
    Applied,
    ReusedVerifiedTarget,
    RecoveredAfterFailure,
    RefusedBeforeEffect,
    RecoveryAmbiguous,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PersonalMarketplaceUpdateReport {
    pub(crate) disposition: PersonalMarketplaceUpdateDisposition,
    pub(crate) failed_stage: Option<PersonalMarketplaceUpdateStage>,
}

pub(crate) fn execute_personal_marketplace_update(
    authority: &PersonalMarketplaceUpdateAuthority,
    effects: &mut impl PersonalMarketplaceUpdateEffects,
) -> PersonalMarketplaceUpdateReport {
    if authority.validate().is_err() {
        return report(
            PersonalMarketplaceUpdateDisposition::RefusedBeforeEffect,
            Some(PersonalMarketplaceUpdateStage::InitialObservation),
        );
    }
    let initial = match effects.observe(authority) {
        Ok(observation) => observation,
        Err(_) => {
            return report(
                PersonalMarketplaceUpdateDisposition::RefusedBeforeEffect,
                Some(PersonalMarketplaceUpdateStage::InitialObservation),
            );
        }
    };
    if initial.exact_target() {
        return report(
            PersonalMarketplaceUpdateDisposition::ReusedVerifiedTarget,
            None,
        );
    }
    if initial.target_tree_only() {
        return rollback(
            authority,
            effects,
            PersonalMarketplaceUpdateStage::InterruptedAfterMaterialization,
        );
    }
    if !initial.exact_prior() {
        return report(
            PersonalMarketplaceUpdateDisposition::RefusedBeforeEffect,
            Some(PersonalMarketplaceUpdateStage::InitialObservation),
        );
    }
    if effects.materialize_target(authority).is_err() {
        return rollback(
            authority,
            effects,
            PersonalMarketplaceUpdateStage::MaterializeTarget,
        );
    }
    let materialized = effects.observe(authority);
    if !matches!(materialized, Ok(observation) if observation.target_tree_only()) {
        return rollback(
            authority,
            effects,
            PersonalMarketplaceUpdateStage::ReconcileMaterializedTarget,
        );
    }
    let install_result = effects.install_target(authority);
    let observed_target = effects.observe(authority);
    if matches!(observed_target, Ok(observation) if observation.exact_target()) {
        return report(PersonalMarketplaceUpdateDisposition::Applied, None);
    }
    rollback(
        authority,
        effects,
        if install_result.is_err() {
            PersonalMarketplaceUpdateStage::InstallTarget
        } else {
            PersonalMarketplaceUpdateStage::ReconcileTarget
        },
    )
}

fn rollback(
    authority: &PersonalMarketplaceUpdateAuthority,
    effects: &mut impl PersonalMarketplaceUpdateEffects,
    failed_stage: PersonalMarketplaceUpdateStage,
) -> PersonalMarketplaceUpdateReport {
    if matches!(effects.observe(authority), Ok(observation) if observation.exact_prior()) {
        return report(
            PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure,
            Some(failed_stage),
        );
    }
    if effects.restore_prior_tree(authority).is_err() {
        return recovered_or_ambiguous(
            authority,
            effects,
            failed_stage,
            PersonalMarketplaceUpdateStage::RestorePriorTree,
        );
    }
    if !matches!(effects.observe(authority), Ok(observation) if observation.prior_tree_is_restored())
    {
        return report(
            PersonalMarketplaceUpdateDisposition::RecoveryAmbiguous,
            Some(PersonalMarketplaceUpdateStage::RestorePriorTree),
        );
    }
    if effects.reinstall_prior(authority).is_err() {
        return recovered_or_ambiguous(
            authority,
            effects,
            failed_stage,
            PersonalMarketplaceUpdateStage::ReinstallPrior,
        );
    }
    recovered_or_ambiguous(
        authority,
        effects,
        failed_stage,
        PersonalMarketplaceUpdateStage::ReconcilePrior,
    )
}

fn recovered_or_ambiguous(
    authority: &PersonalMarketplaceUpdateAuthority,
    effects: &mut impl PersonalMarketplaceUpdateEffects,
    original_failure: PersonalMarketplaceUpdateStage,
    recovery_failure: PersonalMarketplaceUpdateStage,
) -> PersonalMarketplaceUpdateReport {
    match effects.observe(authority) {
        Ok(observation) if observation.exact_prior() => report(
            PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure,
            Some(original_failure),
        ),
        _ => report(
            PersonalMarketplaceUpdateDisposition::RecoveryAmbiguous,
            Some(recovery_failure),
        ),
    }
}

fn report(
    disposition: PersonalMarketplaceUpdateDisposition,
    failed_stage: Option<PersonalMarketplaceUpdateStage>,
) -> PersonalMarketplaceUpdateReport {
    PersonalMarketplaceUpdateReport {
        disposition,
        failed_stage,
    }
}

fn digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_product::lifecycle::{PackageAuthority, Version};

    #[derive(Clone, Copy)]
    enum Failure {
        MaterializeAfterEffect,
        InstallBeforeEffect,
        InstallAfterEffect,
        SubstituteAfterInstall,
        Restore,
        ReinstallBeforeEffect,
        ReinstallAfterEffect,
    }

    struct Effects {
        observation: PersonalMarketplaceUpdateObservation,
        failure: Option<Failure>,
        materializations: usize,
        target_installs: usize,
        restorations: usize,
        prior_installs: usize,
    }

    impl Effects {
        fn prior() -> Self {
            Self::new(exact(PersonalMarketplaceAuthorityMatch::Prior))
        }

        fn target() -> Self {
            Self::new(exact(PersonalMarketplaceAuthorityMatch::Target))
        }

        fn interrupted() -> Self {
            let mut observation = exact(PersonalMarketplaceAuthorityMatch::Prior);
            observation.source_tree = PersonalMarketplaceAuthorityMatch::Target;
            Self::new(observation)
        }

        fn new(observation: PersonalMarketplaceUpdateObservation) -> Self {
            Self {
                observation,
                failure: None,
                materializations: 0,
                target_installs: 0,
                restorations: 0,
                prior_installs: 0,
            }
        }

        fn fail(mut self, failure: Failure) -> Self {
            self.failure = Some(failure);
            self
        }
    }

    impl PersonalMarketplaceUpdateEffects for Effects {
        fn observe(
            &mut self,
            _authority: &PersonalMarketplaceUpdateAuthority,
        ) -> Result<PersonalMarketplaceUpdateObservation, &'static str> {
            Ok(self.observation)
        }

        fn materialize_target(
            &mut self,
            _authority: &PersonalMarketplaceUpdateAuthority,
        ) -> Result<(), &'static str> {
            self.materializations += 1;
            self.observation.source_tree = PersonalMarketplaceAuthorityMatch::Target;
            if matches!(self.failure, Some(Failure::MaterializeAfterEffect)) {
                Err("interrupted after materialization")
            } else {
                Ok(())
            }
        }

        fn install_target(
            &mut self,
            _authority: &PersonalMarketplaceUpdateAuthority,
        ) -> Result<(), &'static str> {
            self.target_installs += 1;
            if matches!(self.failure, Some(Failure::InstallBeforeEffect)) {
                return Err("install did not start");
            }
            self.observation = exact(PersonalMarketplaceAuthorityMatch::Target);
            if matches!(self.failure, Some(Failure::SubstituteAfterInstall)) {
                self.observation.cache = PersonalMarketplaceAuthorityMatch::Other;
            }
            if matches!(self.failure, Some(Failure::InstallAfterEffect)) {
                Err("command result lost after effect")
            } else {
                Ok(())
            }
        }

        fn restore_prior_tree(
            &mut self,
            _authority: &PersonalMarketplaceUpdateAuthority,
        ) -> Result<(), &'static str> {
            self.restorations += 1;
            if matches!(self.failure, Some(Failure::Restore)) {
                return Err("prior tree restoration failed");
            }
            self.observation.source_tree = PersonalMarketplaceAuthorityMatch::Prior;
            Ok(())
        }

        fn reinstall_prior(
            &mut self,
            _authority: &PersonalMarketplaceUpdateAuthority,
        ) -> Result<(), &'static str> {
            self.prior_installs += 1;
            if matches!(self.failure, Some(Failure::ReinstallBeforeEffect)) {
                return Err("prior reinstall did not start");
            }
            self.observation = exact(PersonalMarketplaceAuthorityMatch::Prior);
            if matches!(self.failure, Some(Failure::ReinstallAfterEffect)) {
                Err("prior reinstall result lost after effect")
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn exact_prior_applies_and_exact_target_replay_is_zero_effect() {
        let authority = authority();
        let mut first = Effects::prior();
        let applied = execute_personal_marketplace_update(&authority, &mut first);
        assert_eq!(
            applied.disposition,
            PersonalMarketplaceUpdateDisposition::Applied
        );
        assert_eq!((first.materializations, first.target_installs), (1, 1));

        let mut replay = Effects::target();
        let reused = execute_personal_marketplace_update(&authority, &mut replay);
        assert_eq!(
            reused.disposition,
            PersonalMarketplaceUpdateDisposition::ReusedVerifiedTarget
        );
        assert_eq!(
            (
                replay.materializations,
                replay.target_installs,
                replay.restorations
            ),
            (0, 0, 0)
        );
    }

    #[test]
    fn interruption_after_materialization_restores_and_reinstalls_exact_prior() {
        let authority = authority();
        let mut effects = Effects::interrupted();
        let report = execute_personal_marketplace_update(&authority, &mut effects);
        assert_eq!(
            report,
            PersonalMarketplaceUpdateReport {
                disposition: PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure,
                failed_stage: Some(PersonalMarketplaceUpdateStage::InterruptedAfterMaterialization),
            }
        );
        assert_eq!((effects.restorations, effects.prior_installs), (1, 1));
        assert!(effects.observation.exact_prior());
    }

    #[test]
    fn every_target_failure_recovers_or_accepts_an_independently_observed_target() {
        for failure in [
            Failure::MaterializeAfterEffect,
            Failure::InstallBeforeEffect,
            Failure::SubstituteAfterInstall,
        ] {
            let authority = authority();
            let mut effects = Effects::prior().fail(failure);
            let report = execute_personal_marketplace_update(&authority, &mut effects);
            assert_eq!(
                report.disposition,
                PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure
            );
            assert!(effects.observation.exact_prior());
        }

        let authority = authority();
        let mut lost_result = Effects::prior().fail(Failure::InstallAfterEffect);
        let report = execute_personal_marketplace_update(&authority, &mut lost_result);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::Applied
        );
        assert!(lost_result.observation.exact_target());
    }

    #[test]
    fn rollback_failure_never_claims_prior_or_target_success() {
        let update_authority = authority();
        let mut failed_restore = Effects::prior().fail(Failure::Restore);
        failed_restore.observation.source_tree = PersonalMarketplaceAuthorityMatch::Target;
        let report = execute_personal_marketplace_update(&update_authority, &mut failed_restore);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RecoveryAmbiguous
        );

        let update_authority = authority();
        let mut prior_already_exact = Effects::prior().fail(Failure::ReinstallBeforeEffect);
        prior_already_exact.observation.source_tree = PersonalMarketplaceAuthorityMatch::Target;
        let report =
            execute_personal_marketplace_update(&update_authority, &mut prior_already_exact);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure
        );
        assert!(prior_already_exact.observation.exact_prior());

        let authority = authority();
        let mut lost_result = Effects::prior().fail(Failure::ReinstallAfterEffect);
        lost_result.observation.source_tree = PersonalMarketplaceAuthorityMatch::Target;
        let report = execute_personal_marketplace_update(&authority, &mut lost_result);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure
        );
        assert!(lost_result.observation.exact_prior());
    }

    #[test]
    fn substituted_or_mixed_initial_state_refuses_before_effect() {
        let authority = authority();
        let mut observation = exact(PersonalMarketplaceAuthorityMatch::Prior);
        observation.cache = PersonalMarketplaceAuthorityMatch::Other;
        let mut effects = Effects::new(observation);
        let report = execute_personal_marketplace_update(&authority, &mut effects);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RefusedBeforeEffect
        );
        assert_eq!(
            (
                effects.materializations,
                effects.target_installs,
                effects.restorations,
                effects.prior_installs,
            ),
            (0, 0, 0, 0)
        );
    }

    fn exact(value: PersonalMarketplaceAuthorityMatch) -> PersonalMarketplaceUpdateObservation {
        PersonalMarketplaceUpdateObservation {
            marketplace_root_bound: true,
            source_tree: value,
            installed: value,
            cache: value,
            runtime: value,
            registry: value,
        }
    }

    fn authority() -> PersonalMarketplaceUpdateAuthority {
        let prior = PriorInstalledAuthority::new(
            PackageAuthority {
                version: Version::parse("0.0.36").unwrap(),
                package_sha256: sha('1'),
                inventory_sha256: sha('2'),
                candidate_id: sha('3'),
            },
            sha('4'),
        )
        .unwrap();
        PersonalMarketplaceUpdateAuthority::new(sha('5'), prior, sha('6'), sha('7'), sha('8'))
            .unwrap()
    }

    fn sha(seed: char) -> String {
        format!("sha256:{}", seed.to_string().repeat(64))
    }
}
