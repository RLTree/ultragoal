use super::*;

impl ReadConfinement {
    pub(crate) fn bind_records(
        root: &RootAnchor,
        sources: &[RepoPath],
    ) -> Result<Vec<RoutineReadSource>, RoutineError> {
        if sources.is_empty() {
            return Ok(Vec::new());
        }
        #[cfg(not(unix))]
        {
            let _ = root;
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            root.validate()?;
            let mut consumed = 0_u64;
            let mut records = Vec::with_capacity(sources.len());
            for source in sources {
                let anchor = open_read_source(root, source)?;
                consumed = consumed
                    .checked_add(anchor.record.byte_length)
                    .filter(|total| *total <= MAX_READ_SOURCE_BYTES)
                    .ok_or_else(|| mediator_error("mediator-read-source-budget-exceeded"))?;
                records.push(anchor.record);
            }
            root.validate()?;
            Ok(records)
        }
    }

    pub(crate) fn open_bound(
        root: &RootAnchor,
        expected: &[RoutineReadSource],
    ) -> Result<Self, RoutineError> {
        if expected.is_empty() {
            return Ok(Self {
                #[cfg(unix)]
                sources: Vec::new(),
            });
        }
        #[cfg(not(unix))]
        {
            let _ = root;
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            root.validate()?;
            let mut consumed = 0_u64;
            let mut sources = Vec::with_capacity(expected.len());
            for expected in expected {
                let anchor = open_read_source(root, &expected.relative_path)?;
                consumed = consumed
                    .checked_add(anchor.record.byte_length)
                    .filter(|total| *total <= MAX_READ_SOURCE_BYTES)
                    .ok_or_else(|| mediator_error("mediator-read-source-budget-exceeded"))?;
                if !read_source_record_matches(expected, &anchor) {
                    return Err(RoutineError::new(
                        RoutineErrorId::ConcurrentMutation,
                        "mediator-read-source-binding-stale",
                        None,
                    ));
                }
                sources.push(anchor);
            }
            let confinement = Self { sources };
            confinement.validate(root)?;
            Ok(confinement)
        }
    }

    pub(crate) fn absolute_sources(&self) -> Vec<&Path> {
        #[cfg(not(unix))]
        {
            Vec::new()
        }
        #[cfg(unix)]
        {
            self.sources
                .iter()
                .map(|source| source.path.as_path())
                .collect()
        }
    }

    pub(crate) fn validate(&self, root: &RootAnchor) -> Result<(), RoutineError> {
        #[cfg(not(unix))]
        {
            let _ = root;
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            root.validate()?;
            for source in &self.sources {
                for (held, expected) in source.ancestors.iter().zip(source.record.ancestors.iter())
                {
                    let current = ObjectIdentity::from(&held.file.metadata().map_err(|_| {
                        mediator_error("mediator-read-source-ancestor-metadata-failed")
                    })?);
                    if !directory_object_matches(held.identity, current)
                        || !ancestor_matches(expected, current)
                    {
                        return Err(RoutineError::new(
                            RoutineErrorId::ConcurrentMutation,
                            "mediator-read-source-ancestor-changed",
                            None,
                        ));
                    }
                }
                let held = ObjectIdentity::from(
                    &source
                        .file
                        .metadata()
                        .map_err(|_| mediator_error("mediator-read-source-metadata-failed"))?,
                );
                let mut reader = source
                    .file
                    .try_clone()
                    .map_err(|_| mediator_error("mediator-read-source-open-failed"))?;
                reader
                    .seek(SeekFrom::Start(0))
                    .map_err(|_| mediator_error("mediator-read-source-read-failed"))?;
                let digest = digest_reader(&mut reader, source.record.byte_length)?;
                if held != source.identity || !source_matches(&source.record, held, &digest) {
                    return Err(RoutineError::new(
                        RoutineErrorId::ConcurrentMutation,
                        "mediator-read-source-changed",
                        None,
                    ));
                }
                let current = open_read_source(root, &source.record.relative_path)?;
                if !read_source_record_matches(&source.record, &current) {
                    return Err(RoutineError::new(
                        RoutineErrorId::ConcurrentMutation,
                        "mediator-read-source-replaced",
                        None,
                    ));
                }
            }
            root.validate()?;
            Ok(())
        }
    }
}
