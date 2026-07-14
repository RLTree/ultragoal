impl ResearchAudit {
    pub fn audit(sources: &[ResearchSource], proposals: &[LawChangeProposal]) -> Self {
        let current_epoch_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs());
        Self::audit_with_current_time(sources, proposals, current_epoch_seconds)
    }

    #[cfg(test)]
    pub(crate) fn test_only_audit_at(
        sources: &[ResearchSource],
        proposals: &[LawChangeProposal],
        current_epoch_seconds: u64,
    ) -> Self {
        Self::audit_with_current_time(sources, proposals, Some(current_epoch_seconds))
    }

    #[cfg(test)]
    pub(crate) fn test_only_audit_with_clock_failure(
        sources: &[ResearchSource],
        proposals: &[LawChangeProposal],
    ) -> Self {
        Self::audit_with_current_time(sources, proposals, None)
    }

    fn audit_with_current_time(
        sources: &[ResearchSource],
        proposals: &[LawChangeProposal],
        current_epoch_seconds: Option<u64>,
    ) -> Self {
        let mut findings = Vec::new();
        let Some(current_epoch_seconds) = current_epoch_seconds else {
            findings.push(finding("research-clock-unavailable", None, None));
            return Self::with(findings, Vec::new());
        };
        if current_epoch_seconds == 0 || current_epoch_seconds == u64::MAX {
            findings.push(finding("research-clock-invalid", None, None));
            return Self::with(findings, Vec::new());
        }
        if sources.is_empty() || sources.len() > MAX_RESEARCH_SOURCES {
            findings.push(finding("research-source-count-out-of-bounds", None, None));
        }
        if proposals.is_empty() || proposals.len() > MAX_PROPOSALS {
            findings.push(finding("research-proposal-count-out-of-bounds", None, None));
        }
        if !findings.is_empty() {
            return Self::with(findings, Vec::new());
        }

        let mut source_id_counts = BTreeMap::new();
        let mut snapshot_digest_counts = BTreeMap::new();
        for source in sources {
            *source_id_counts
                .entry(source.source_id())
                .or_insert(0_usize) += 1;
            *snapshot_digest_counts
                .entry(source.snapshot.digest_sha256())
                .or_insert(0_usize) += 1;
        }
        let mut valid_sources = BTreeMap::new();
        for source in sources {
            let source_reference = valid_reference(source.source_id());
            let shape_valid = source.validate_shape().is_ok();
            if !shape_valid {
                findings.push(finding("research-source-invalid", source_reference, None));
            }
            let authority_bound = source.has_adopted_authority_binding();
            if !authority_bound {
                findings.push(finding(
                    "research-source-authority-binding-required",
                    source_reference,
                    None,
                ));
            }
            let current = source.record.is_current_at(current_epoch_seconds);
            if !current {
                findings.push(finding("research-source-stale", source_reference, None));
            }
            let temporal_scope_supported = !source.record.has_externally_untrusted_stable_fact();
            if !temporal_scope_supported {
                findings.push(finding(
                    "research-source-stable-fact-authority-required",
                    source_reference,
                    None,
                ));
            }
            let primary_supported = !source.record.requires_current_primary_source()
                || source.class().supports_current_capability_decision();
            if !primary_supported {
                findings.push(finding(
                    "research-source-current-primary-required",
                    source_reference,
                    None,
                ));
            }
            let duplicate = source_id_counts
                .get(source.source_id())
                .is_some_and(|count| *count > 1)
                || snapshot_digest_counts
                    .get(source.snapshot.digest_sha256())
                    .is_some_and(|count| *count > 1);
            if duplicate {
                findings.push(finding("research-source-duplicate", source_reference, None));
            }
            if shape_valid
                && authority_bound
                && current
                && temporal_scope_supported
                && primary_supported
                && !duplicate
            {
                valid_sources.insert(source.source_id(), source);
            }
        }

        let mut proposal_id_counts = BTreeMap::new();
        for proposal in proposals {
            *proposal_id_counts
                .entry(proposal.proposal_id.as_str())
                .or_insert(0_usize) += 1;
        }
        let mut eligible_proposals = Vec::new();
        for proposal in proposals {
            let proposal_reference = valid_reference(&proposal.proposal_id);
            let shape_valid = proposal.validate().is_ok();
            if !shape_valid {
                findings.push(finding(
                    "research-proposal-invalid",
                    None,
                    proposal_reference,
                ));
            }
            let duplicate = proposal_id_counts
                .get(proposal.proposal_id.as_str())
                .is_some_and(|count| *count > 1);
            if duplicate {
                findings.push(finding(
                    "research-proposal-duplicate",
                    None,
                    proposal_reference,
                ));
            }

            let mut supported_laws = BTreeSet::new();
            let support_valid = proposal.supporting_source_ids.iter().all(|source_id| {
                valid_sources.get(source_id.as_str()).is_some_and(|source| {
                    if source
                        .supports_proposal_ids()
                        .contains(&proposal.proposal_id)
                    {
                        supported_laws.extend(source.record.mapped_law_ids().iter().cloned());
                        true
                    } else {
                        false
                    }
                })
            });
            let mapped_laws_valid = proposal.mapped_law_ids.is_subset(&supported_laws);
            if !support_valid || !mapped_laws_valid {
                findings.push(finding(
                    "research-proposal-support-invalid",
                    None,
                    proposal_reference,
                ));
            }
            if shape_valid && !duplicate && support_valid && mapped_laws_valid {
                eligible_proposals.push(proposal.normalized());
            }
        }

        Self::with(findings, eligible_proposals)
    }

    pub fn eligible_proposals(&self) -> &[LawChangeProposal] {
        &self.eligible_proposals
    }

    pub fn findings(&self) -> &[ResearchFinding] {
        &self.findings
    }

    pub fn authority_effect(&self) -> &'static str {
        self.authority_effect.as_str()
    }
}
