impl RetirementDecision {
    fn reconcile_decision(
        plan: &MigrationPlan,
        target_id: &str,
        current: &MigrationInventory,
        review: &RetirementReview,
        replacement_authority: &mut dyn ReplacementEvidenceAuthority,
        review_authority: &mut dyn RetirementReviewAuthority,
        destructive: Option<(
            &DestructiveAuthorization,
            &mut dyn DestructiveEffectAuthority,
        )>,
    ) -> Self {
        let mut reasons = Vec::new();
        if plan.verify_current(current).is_err() {
            reasons.push("migration-plan-not-current".to_owned());
        }
        if !replacement_plan_ledger_is_current(plan, current, replacement_authority) {
            reasons.push("migration-replacement-ledger-not-current".to_owned());
        }
        let target = plan
            .targets
            .iter()
            .find(|target| target.target_id == target_id);
        if target.is_none() {
            reasons.push("migration-retirement-target-unknown".to_owned());
        }
        let review_is_current = target
            .map(|target| {
                retirement_review_is_current(plan, target, current, review, review_authority)
            })
            .unwrap_or(false);
        if !review_is_current {
            reasons.push("migration-retirement-review-invalid".to_owned());
        }
        if let Some(target) = target {
            let route = plan
                .routes
                .iter()
                .find(|route| route.route_id == target.route_id);
            let replacement = plan.replacement_evidence(target);
            let replacement_is_current = route.zip(replacement).is_some_and(|(route, evidence)| {
                evidence
                    .validate_with_authority(current, route, replacement_authority)
                    .is_ok()
            });
            if !replacement_is_current {
                reasons.push("migration-replacement-evidence-invalid".to_owned());
            }
            if !target.active_readers.is_empty() {
                reasons.push("migration-active-readers-remain".to_owned());
            }
            if !target.active_writers.is_empty() {
                reasons.push("migration-active-writers-remain".to_owned());
            }
            if !target.public_routes.is_empty() {
                reasons.push("migration-public-routes-remain".to_owned());
            }
            if !target.generated_outputs.is_empty() {
                reasons.push("migration-generated-authority-remains".to_owned());
            }
            if target.observed_invocations != 0 || !target.compatibility_window_complete {
                reasons.push("migration-compatibility-window-open".to_owned());
            }
            if target.source_status == SurfaceStatus::Retired {
                reasons.push("migration-target-already-retired".to_owned());
            }
            if route.zip(replacement).is_none_or(|(route, evidence)| {
                evidence.observation.validate(route).is_err()
                    || !valid_sha256(&target.replacement_summary_sha256)
                    || replacement_summary_commitment(evidence) != target.replacement_summary_sha256
            }) {
                reasons.push("migration-replacement-behavior-unverified".to_owned());
            }
        }

        let mut destructive_authorization_id = None;
        let mut destructive_to_consume = None;
        match (review.od009_decision, destructive) {
            (Od009Decision::PreservePhysicalArtifact, Some(_)) => {
                reasons.push("migration-conflicting-destructive-authorization".to_owned());
            }
            (Od009Decision::RequestPhysicalDeletion, None) => {
                reasons.push("migration-destructive-authority-required".to_owned());
            }
            (Od009Decision::RequestPhysicalDeletion, Some((authorization, authority))) => {
                let authorization_is_current = target
                    .map(|target| {
                        destructive_authorization_is_current(DestructiveAuthorizationCurrentness {
                            plan,
                            target,
                            current,
                            review,
                            authorization,
                            review_authority,
                            authority,
                        })
                    })
                    .unwrap_or(false);
                if !authorization_is_current {
                    reasons.push("migration-destructive-authorization-invalid".to_owned());
                } else {
                    destructive_authorization_id = Some(authorization.authorization_id.clone());
                    destructive_to_consume = Some((authorization, authority));
                }
            }
            (Od009Decision::PreservePhysicalArtifact, None) => {}
        }

        if reasons.is_empty() {
            let ledger_binding = target
                .and_then(|target| ReplacementLedgerBinding::issue(plan, target, current).ok());
            let final_claim = ledger_binding.as_ref().and_then(|binding| {
                replacement_authority
                    .claim_final_reconciliation(binding)
                    .ok()
            });
            let ledger_is_current_before_review = match (&ledger_binding, &final_claim) {
                (Some(binding), Some(claim)) => {
                    valid_sha256(claim)
                        && replacement_plan_ledger_is_current(plan, current, replacement_authority)
                        && replacement_authority.verify_final_claim(binding, claim)
                }
                _ => false,
            };
            if !ledger_is_current_before_review {
                reasons.push("migration-replacement-ledger-invalid-or-replayed".to_owned());
            } else {
                let review_consumed = review_authority.verify_and_consume(
                    &review.binding_sha256,
                    &review.review_id,
                    &review.attestation_sha256,
                );
                let review_remained_current = target
                    .map(|target| {
                        retirement_review_is_current(
                            plan,
                            target,
                            current,
                            review,
                            review_authority,
                        )
                    })
                    .unwrap_or(false);
                let replacement_remained_current = match (&ledger_binding, &final_claim) {
                    (Some(binding), Some(claim)) => {
                        replacement_plan_ledger_is_current(plan, current, replacement_authority)
                            && replacement_authority.verify_final_claim(binding, claim)
                    }
                    _ => false,
                };
                if !replacement_remained_current {
                    reasons.push("migration-replacement-ledger-drifted".to_owned());
                }
                if !review_consumed || !review_remained_current {
                    reasons.push("migration-retirement-review-invalid-or-replayed".to_owned());
                } else if replacement_remained_current {
                    if let Some((authorization, authority)) = destructive_to_consume {
                        let replacement_is_current_before_effect =
                            match (&ledger_binding, &final_claim) {
                                (Some(binding), Some(claim)) => {
                                    replacement_plan_ledger_is_current(
                                        plan,
                                        current,
                                        replacement_authority,
                                    ) && replacement_authority.verify_final_claim(binding, claim)
                                }
                                _ => false,
                            };
                        if !replacement_is_current_before_effect {
                            reasons.push("migration-replacement-ledger-drifted".to_owned());
                        } else {
                            let authorization_consumed = authority.verify_and_consume(
                                &authorization.binding_sha256,
                                &authorization.authorization_id,
                                &authorization.attestation_sha256,
                            );
                            let authorization_remained_current = target
                                .map(|target| {
                                    destructive_authorization_is_current(
                                        DestructiveAuthorizationCurrentness {
                                            plan,
                                            target,
                                            current,
                                            review,
                                            authorization,
                                            review_authority,
                                            authority,
                                        },
                                    )
                                })
                                .unwrap_or(false);
                            let replacement_remained_current = match (&ledger_binding, &final_claim)
                            {
                                (Some(binding), Some(claim)) => {
                                    replacement_plan_ledger_is_current(
                                        plan,
                                        current,
                                        replacement_authority,
                                    ) && replacement_authority.verify_final_claim(binding, claim)
                                }
                                _ => false,
                            };
                            if !replacement_remained_current {
                                reasons.push("migration-replacement-ledger-drifted".to_owned());
                            }
                            if !authorization_consumed || !authorization_remained_current {
                                reasons.push(
                                    "migration-destructive-authorization-invalid-or-replayed"
                                        .to_owned(),
                                );
                            }
                        }
                    }
                }
            }
        }
        reasons.sort();
        reasons.dedup();
        let status = if !reasons.is_empty() {
            RetirementStatus::Blocked
        } else if review.od009_decision == Od009Decision::RequestPhysicalDeletion {
            RetirementStatus::DestructiveRetirementCandidate
        } else {
            RetirementStatus::NonAuthoritativePreservationCandidate
        };
        Self {
            target_id: target_id.to_owned(),
            live_context_id: current.live_context_id.clone(),
            candidate_id: current.candidate_id.clone(),
            reviewer_id: review.reviewer_id.clone(),
            review_id: review.review_id.clone(),
            destructive_authorization_id,
            status,
            reasons,
            plan_sha256: plan.plan_sha256.clone(),
            claim_ceiling: "migration_candidate_not_adoption_or_retirement".to_owned(),
        }
    }
}
