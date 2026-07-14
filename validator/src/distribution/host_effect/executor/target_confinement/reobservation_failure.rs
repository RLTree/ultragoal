struct InventoryObservationRequest<'a> {
    target_name: &'a str,
    expected_temporary_name: &'a str,
    expectation: PublicationExpectation,
    ledger_head: super::super::HostEffectLedgerHead,
    acknowledgement: Option<PublicationAcknowledgementIdentity>,
    data_synced: bool,
}

impl ConfinedHostEffectTarget {
    pub(super) fn reobserve_failure(
        &self,
        failure: &PublicationFailure,
        ledger_head: super::super::HostEffectLedgerHead,
    ) -> Result<
        (PublicationInventoryObservation, PublicationClassification),
        HostEffectExecutorFailure,
    > {
        self.observe_inventory(InventoryObservationRequest {
            target_name: &failure.target_name,
            expected_temporary_name: &failure.temporary_name,
            expectation: failure.expectation.clone(),
            ledger_head,
            acknowledgement: None,
            data_synced: false,
        })
    }

    fn observe_inventory(
        &self,
        request: InventoryObservationRequest<'_>,
    ) -> Result<
        (PublicationInventoryObservation, PublicationClassification),
        HostEffectExecutorFailure,
    > {
        let InventoryObservationRequest {
            target_name,
            expected_temporary_name,
            expectation,
            ledger_head,
            acknowledgement,
            data_synced,
        } = request;
        self.revalidate_anchor()?;
        let before =
            ObjectIdentity::capture(&self.anchor.directory.metadata().map_err(|_| path_swap())?)?;
        let scan_generation = observation_generation(&before);
        let target = self
            .observe_object(target_name, scan_generation, data_synced)?
            .observation;
        let temporary_prefix = format!(".{target_name}.");
        let mut temporary_objects = Vec::new();
        for name in self.names()? {
            if name.starts_with(&temporary_prefix) && name.ends_with(".tmp") {
                temporary_objects.push(
                    self.observe_object(&name, scan_generation, data_synced)?
                        .observation,
                );
            }
        }
        // If the expected name disappeared but another collision appeared,
        // the complete matching inventory is still passed to classification.
        let _ = expected_temporary_name;
        let after =
            ObjectIdentity::capture(&self.anchor.directory.metadata().map_err(|_| path_swap())?)?;
        let after_generation = if before == after {
            scan_generation
        } else {
            scan_generation.saturating_add(1)
        };
        let observation =
            PublicationInventoryObservation::new(PublicationInventoryObservationRequest {
                scan_generation_before: scan_generation,
                scan_generation_after: after_generation,
                target,
                temporary_objects,
                expectation,
                current_ledger_head: ledger_head,
                acknowledgement,
            })
            .map_err(|_| unsafe_object())?;
        let classification = observation.classify().map_err(|_| unsafe_object())?;
        Ok((observation, classification))
    }

    fn observe_object(
        &self,
        name: &str,
        generation: u64,
        data_synced: bool,
    ) -> Result<ObservedObject, HostEffectExecutorFailure> {
        let name_c = component(name)?;
        let Some(metadata) = stat_at(
            self.anchor.directory.as_raw_fd(),
            &name_c,
            libc::AT_SYMLINK_NOFOLLOW,
        )?
        else {
            return Ok(ObservedObject {
                kind: PublicationObjectKind::Missing,
                observation: PublicationObjectObservation::missing(name.to_owned(), generation)
                    .map_err(|_| unsafe_object())?,
            });
        };
        let identity = ObjectIdentity::capture(&metadata)?;
        let kind = identity.kind();
        if kind != PublicationObjectKind::Regular {
            return Ok(ObservedObject {
                kind,
                observation: PublicationObjectObservation::new(
                    PublicationObjectObservationRequest {
                        name: name.to_owned(),
                        kind,
                        byte_length: identity.length,
                        mode: identity.mode,
                        hard_links: identity.links,
                        content_sha256: None,
                        object_generation: generation,
                        data_synced: false,
                    },
                )
                .map_err(|_| unsafe_object())?,
            });
        }
        let snapshot = self
            .read_regular(name, MAX_PUBLICATION_BYTES)?
            .ok_or_else(target_substitution)?;
        Ok(ObservedObject {
            kind,
            observation: PublicationObjectObservation::new(PublicationObjectObservationRequest {
                name: name.to_owned(),
                kind,
                byte_length: snapshot.bytes.len() as u64,
                mode: snapshot.identity.mode,
                hard_links: snapshot.identity.links,
                content_sha256: Some(digest_bytes(&snapshot.bytes)),
                object_generation: generation,
                data_synced,
            })
            .map_err(|_| unsafe_object())?,
        })
    }

    fn read_regular(
        &self,
        name: &str,
        maximum: usize,
    ) -> Result<Option<FileSnapshot>, HostEffectExecutorFailure> {
        let name_c = component(name)?;
        let descriptor = unsafe {
            libc::openat(
                self.anchor.directory.as_raw_fd(),
                name_c.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return if last_errno() == Some(libc::ENOENT) {
                Ok(None)
            } else {
                Err(unsafe_object())
            };
        }
        let mut file = unsafe { File::from_raw_fd(descriptor) };
        let before = ObjectIdentity::capture(&file.metadata().map_err(|_| unsafe_object())?)?;
        if !before.regular()
            || before.links != 1
            || before.device != self.anchor.identity.device
            || before.uid != unsafe { libc::geteuid() }
            || before.length > maximum as u64
        {
            return Err(unsafe_object());
        }
        let mut bytes = Vec::with_capacity(before.length as usize);
        Read::by_ref(&mut file)
            .take(maximum as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| unsafe_object())?;
        let after = ObjectIdentity::capture(&file.metadata().map_err(|_| unsafe_object())?)?;
        let named = stat_at(
            self.anchor.directory.as_raw_fd(),
            &name_c,
            libc::AT_SYMLINK_NOFOLLOW,
        )?
        .ok_or_else(target_substitution)?;
        let named = ObjectIdentity::capture(&named)?;
        if bytes.len() > maximum
            || before != after
            || before != named
            || bytes.len() as u64 != before.length
        {
            return Err(target_substitution());
        }
        Ok(Some(FileSnapshot {
            identity: before,
            bytes,
        }))
    }
}
