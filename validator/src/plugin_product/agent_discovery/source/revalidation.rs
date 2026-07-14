use super::*;

impl SourceAgentCatalog {
    pub(crate) fn revalidate(&self) -> Result<(), AgentDiscoveryError> {
        self.revalidate_with_hooks_internal(|| {}, || {})
    }

    fn revalidate_with_hooks_internal<Before, After>(
        &self,
        before_anchored_reads: Before,
        after_anchored_reads: After,
    ) -> Result<(), AgentDiscoveryError>
    where
        Before: FnOnce(),
        After: FnOnce(),
    {
        self.revalidate_anchors()?;
        before_anchored_reads();
        let content_result = self.revalidate_contents();
        after_anchored_reads();
        let anchor_result = self.revalidate_anchors();
        content_result?;
        anchor_result
    }

    #[cfg(test)]
    pub(crate) fn revalidate_with_test_hooks<Before, After>(
        &self,
        before_anchored_reads: Before,
        after_anchored_reads: After,
    ) -> Result<(), AgentDiscoveryError>
    where
        Before: FnOnce(),
        After: FnOnce(),
    {
        self.revalidate_with_hooks_internal(before_anchored_reads, after_anchored_reads)
    }

    #[cfg(test)]
    pub(crate) fn revalidate_anchored_contents_for_test(&self) -> Result<(), AgentDiscoveryError> {
        self.revalidate_contents()
    }

    fn revalidate_anchors(&self) -> Result<(), AgentDiscoveryError> {
        self.root.revalidate()?;
        self.root.revalidate_dir(".codex", &self.codex_root)?;
        self.root
            .revalidate_dir(".codex/agents", &self.agents_root)?;
        self.root
            .revalidate_dir(".codex-plugin", &self.plugin_root)?;
        Ok(())
    }

    fn revalidate_contents(&self) -> Result<(), AgentDiscoveryError> {
        let expected_names = self
            .agents
            .iter()
            .map(|agent| agent.file_name.clone())
            .collect::<BTreeSet<_>>();
        let expected_plugin_names = [OsString::from("plugin.json")]
            .into_iter()
            .collect::<BTreeSet<_>>();
        if expected_names.len() != EXACT_AGENT_ENTRY_COUNT
            || expected_plugin_names.len() != EXACT_PLUGIN_ENTRY_COUNT
            || !self.agents_root.exact_regular_entries(&expected_names)?
            || !self
                .plugin_root
                .exact_regular_entries(&expected_plugin_names)?
        {
            return Err(changed());
        }
        if !self.plugin_root.same_file(
            OsStr::new("plugin.json"),
            MAX_MANIFEST_BYTES,
            &self.plugin_manifest,
        )? {
            return Err(changed());
        }
        for agent in &self.agents {
            if !self.agents_root.same_file(
                &agent.file_name,
                MAX_DESCRIPTOR_BYTES,
                &agent.secure_file,
            )? {
                return Err(changed());
            }
        }
        Ok(())
    }
}
