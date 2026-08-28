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

    pub(crate) fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub(crate) fn prior(&self) -> &PriorInstalledAuthority {
        &self.prior
    }

    pub(crate) fn prior_tree_sha256(&self) -> &str {
        &self.prior_tree_sha256
    }

    pub(crate) fn target_package_sha256(&self) -> &str {
        &self.target_package_sha256
    }

    pub(crate) fn target_tree_sha256(&self) -> &str {
        &self.target_tree_sha256
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

    fn exact_materialized_pending_install(self) -> bool {
        self.marketplace_root_bound
            && self.source_tree == PersonalMarketplaceAuthorityMatch::Target
            && self.installed == PersonalMarketplaceAuthorityMatch::Target
            && self.cache == PersonalMarketplaceAuthorityMatch::Prior
            && self.runtime == PersonalMarketplaceAuthorityMatch::Other
            && self.registry == PersonalMarketplaceAuthorityMatch::Prior
    }

    fn prior_tree_is_restored(self) -> bool {
        self.marketplace_root_bound && self.source_tree == PersonalMarketplaceAuthorityMatch::Prior
    }
}

pub(crate) trait PersonalMarketplaceUpdateEffects {
    fn cancelled(&self) -> bool {
        false
    }

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
    if initial.exact_materialized_pending_install() {
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
    if effects.cancelled() {
        return report(
            PersonalMarketplaceUpdateDisposition::RefusedBeforeEffect,
            Some(PersonalMarketplaceUpdateStage::MaterializeTarget),
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
    if !matches!(materialized, Ok(observation) if observation.exact_materialized_pending_install())
    {
        return rollback(
            authority,
            effects,
            PersonalMarketplaceUpdateStage::ReconcileMaterializedTarget,
        );
    }
    if effects.cancelled() {
        return rollback(
            authority,
            effects,
            PersonalMarketplaceUpdateStage::InstallTarget,
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
    if effects.cancelled() || effects.reinstall_prior(authority).is_err() {
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
        cancel_before_start: bool,
        cancel_after_materialization: bool,
        cancel_after_restore: bool,
        observations: usize,
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
            Self::new(materialized_pending_install())
        }

        fn new(observation: PersonalMarketplaceUpdateObservation) -> Self {
            Self {
                observation,
                failure: None,
                cancel_before_start: false,
                cancel_after_materialization: false,
                cancel_after_restore: false,
                observations: 0,
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

        fn cancel_before_start(mut self) -> Self {
            self.cancel_before_start = true;
            self
        }

        fn cancel_after_materialization(mut self) -> Self {
            self.cancel_after_materialization = true;
            self
        }

        fn cancel_after_restore(mut self) -> Self {
            self.cancel_after_restore = true;
            self
        }
    }

    impl PersonalMarketplaceUpdateEffects for Effects {
        fn cancelled(&self) -> bool {
            self.cancel_before_start
                || (self.cancel_after_materialization && self.materializations > 0)
                || (self.cancel_after_restore && self.restorations > 0)
        }

        fn observe(
            &mut self,
            _authority: &PersonalMarketplaceUpdateAuthority,
        ) -> Result<PersonalMarketplaceUpdateObservation, &'static str> {
            self.observations += 1;
            Ok(self.observation)
        }

        fn materialize_target(
            &mut self,
            _authority: &PersonalMarketplaceUpdateAuthority,
        ) -> Result<(), &'static str> {
            self.materializations += 1;
            self.observation = materialized_pending_install();
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
            self.observation.installed = PersonalMarketplaceAuthorityMatch::Prior;
            self.observation.runtime =
                if self.observation.cache == PersonalMarketplaceAuthorityMatch::Prior {
                    PersonalMarketplaceAuthorityMatch::Prior
                } else {
                    PersonalMarketplaceAuthorityMatch::Other
                };
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
        assert!(lost_result.observations >= 3);
        assert!(lost_result.observation.exact_target());
    }

    #[test]
    fn cancellation_is_zero_effect_before_start_and_recovers_after_materialization() {
        let authority = authority();
        let mut before = Effects::prior().cancel_before_start();
        let report = execute_personal_marketplace_update(&authority, &mut before);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RefusedBeforeEffect
        );
        assert_eq!(
            (
                before.materializations,
                before.target_installs,
                before.restorations,
                before.prior_installs,
            ),
            (0, 0, 0, 0)
        );

        let mut after = Effects::prior().cancel_after_materialization();
        let report = execute_personal_marketplace_update(&authority, &mut after);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure
        );
        assert_eq!(
            (
                after.materializations,
                after.target_installs,
                after.restorations,
                after.prior_installs,
            ),
            (1, 0, 1, 0)
        );
        assert!(after.observation.exact_prior());
    }

    #[test]
    fn cancellation_during_mixed_recovery_never_claims_success() {
        let authority = authority();
        let mut effects = Effects::prior()
            .fail(Failure::SubstituteAfterInstall)
            .cancel_after_restore();
        let report = execute_personal_marketplace_update(&authority, &mut effects);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RecoveryAmbiguous
        );
        assert_eq!((effects.restorations, effects.prior_installs), (1, 0));
    }

    #[test]
    fn rollback_failure_never_claims_prior_or_target_success() {
        let update_authority = authority();
        let mut failed_restore = Effects::interrupted().fail(Failure::Restore);
        let report = execute_personal_marketplace_update(&update_authority, &mut failed_restore);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RecoveryAmbiguous
        );

        let update_authority = authority();
        let mut prior_already_exact = Effects::interrupted().fail(Failure::ReinstallBeforeEffect);
        let report =
            execute_personal_marketplace_update(&update_authority, &mut prior_already_exact);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure
        );
        assert!(prior_already_exact.observation.exact_prior());

        let authority = authority();
        let mut lost_result = Effects::interrupted().fail(Failure::ReinstallAfterEffect);
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

    #[test]
    fn only_the_exact_real_materialized_shape_is_recoverable() {
        let authority = authority();
        let mut exact_pending = Effects::new(materialized_pending_install());
        let report = execute_personal_marketplace_update(&authority, &mut exact_pending);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure
        );

        for substitute in [
            PersonalMarketplaceUpdateObservation {
                installed: PersonalMarketplaceAuthorityMatch::Prior,
                ..materialized_pending_install()
            },
            PersonalMarketplaceUpdateObservation {
                cache: PersonalMarketplaceAuthorityMatch::Target,
                ..materialized_pending_install()
            },
            PersonalMarketplaceUpdateObservation {
                runtime: PersonalMarketplaceAuthorityMatch::Prior,
                ..materialized_pending_install()
            },
            PersonalMarketplaceUpdateObservation {
                registry: PersonalMarketplaceAuthorityMatch::Target,
                ..materialized_pending_install()
            },
        ] {
            let mut effects = Effects::new(substitute);
            let report = execute_personal_marketplace_update(&authority, &mut effects);
            assert_eq!(
                report.disposition,
                PersonalMarketplaceUpdateDisposition::RefusedBeforeEffect
            );
            assert_eq!(
                (
                    effects.materializations,
                    effects.target_installs,
                    effects.restorations
                ),
                (0, 0, 0)
            );
        }
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

    fn materialized_pending_install() -> PersonalMarketplaceUpdateObservation {
        PersonalMarketplaceUpdateObservation {
            marketplace_root_bound: true,
            source_tree: PersonalMarketplaceAuthorityMatch::Target,
            installed: PersonalMarketplaceAuthorityMatch::Target,
            cache: PersonalMarketplaceAuthorityMatch::Prior,
            runtime: PersonalMarketplaceAuthorityMatch::Other,
            registry: PersonalMarketplaceAuthorityMatch::Prior,
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
