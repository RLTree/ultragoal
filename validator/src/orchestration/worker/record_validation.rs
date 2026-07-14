impl WorkerResultV1 {
    fn validate_records(&self) -> Result<(), OrchestrationError> {
        if self
            .changes
            .iter()
            .chain(self.commands_and_tests.iter())
            .chain(self.findings.iter())
            .any(BTreeMap::is_empty)
        {
            return Err(OrchestrationError::InvalidWorkerResult);
        }
        for record in &self.commands_and_tests {
            let command_ok = record
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| !command.is_empty() && command.len() <= 4096);
            let outcome_present = ["status", "result", "exit_code"]
                .iter()
                .any(|key| record.contains_key(*key));
            if !command_ok || !outcome_present {
                return Err(OrchestrationError::InvalidWorkerResult);
            }
        }
        Ok(())
    }
}

fn parse_unique_paths(values: &[String]) -> Result<BTreeSet<CanonicalPath>, OrchestrationError> {
    let mut paths = Vec::with_capacity(values.len());
    for value in values {
        let path = CanonicalPath::parse(value)?;
        if paths
            .iter()
            .any(|prior: &CanonicalPath| prior.overlaps(&path))
        {
            return Err(OrchestrationError::DuplicateOutput);
        }
        paths.push(path);
    }
    Ok(paths.into_iter().collect())
}

fn all_unique(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
