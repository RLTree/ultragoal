impl ConfinedHostEffectTarget {
    fn publish_inner(
        &self,
        prepared: &PreparedPublication,
        current_ledger_head: &super::super::HostEffectLedgerHead,
    ) -> Result<PublicationInventoryObservation, HostEffectExecutorErrorId> {
        self.revalidate_anchor().map_err(|failure| failure.id())?;
        if self
            .observe_object(&prepared.target_name, 1, false)
            .map_err(|failure| failure.id())?
            .kind
            != PublicationObjectKind::Missing
        {
            return Err(HostEffectExecutorErrorId::RenameRace);
        }
        run_fault(FaultPoint::BeforeTempCreate, &self.anchor.canonical_path)
            .map_err(|_| HostEffectExecutorErrorId::Io)?;
        let temporary = CString::new(prepared.temporary_name.as_str())
            .map_err(|_| HostEffectExecutorErrorId::UnsafeObject)?;
        let descriptor = unsafe {
            libc::openat(
                self.anchor.directory.as_raw_fd(),
                temporary.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                0o600,
            )
        };
        if descriptor < 0 {
            return Err(if last_errno() == Some(libc::EEXIST) {
                HostEffectExecutorErrorId::TempCollision
            } else {
                HostEffectExecutorErrorId::Io
            });
        }
        let mut file = unsafe { File::from_raw_fd(descriptor) };
        let initially_created =
            ObjectIdentity::capture(&file.metadata().map_err(|_| HostEffectExecutorErrorId::Io)?)
                .map_err(|_| HostEffectExecutorErrorId::UnsafeObject)?;
        if !initially_created.regular()
            || initially_created.links != 1
            || initially_created.device != self.anchor.identity.device
            || initially_created.uid != unsafe { libc::geteuid() }
        {
            return Err(HostEffectExecutorErrorId::UnsafeObject);
        }
        run_fault(FaultPoint::BeforeTempWrite, &self.anchor.canonical_path)
            .map_err(|_| HostEffectExecutorErrorId::Io)?;
        file.write_all(&prepared.bytes)
            .map_err(|_| HostEffectExecutorErrorId::Io)?;
        if unsafe { libc::fchmod(file.as_raw_fd(), PUBLICATION_MODE as libc::mode_t) } != 0 {
            return Err(HostEffectExecutorErrorId::Io);
        }
        run_fault(FaultPoint::BeforeTempFsync, &self.anchor.canonical_path)
            .map_err(|_| HostEffectExecutorErrorId::SyncFailure)?;
        file.sync_all()
            .map_err(|_| HostEffectExecutorErrorId::SyncFailure)?;
        let expected_temporary = self
            .read_regular(&prepared.temporary_name, MAX_PUBLICATION_BYTES)
            .map_err(|failure| failure.id())?
            .ok_or(HostEffectExecutorErrorId::RenameRace)?;
        if expected_temporary.bytes != prepared.bytes
            || expected_temporary.identity.device != initially_created.device
            || expected_temporary.identity.inode != initially_created.inode
            || expected_temporary.identity.mode & 0o777 != PUBLICATION_MODE
        {
            return Err(HostEffectExecutorErrorId::RenameRace);
        }
        let created = expected_temporary.identity;
        run_fault(FaultPoint::BeforeRename, &self.anchor.canonical_path)
            .map_err(|_| HostEffectExecutorErrorId::RenameRace)?;
        self.revalidate_anchor()
            .map_err(|_| HostEffectExecutorErrorId::PathSwap)?;
        if self
            .observe_object(&prepared.target_name, 1, false)
            .map_err(|failure| failure.id())?
            .kind
            != PublicationObjectKind::Missing
        {
            return Err(HostEffectExecutorErrorId::RenameRace);
        }
        if !self
            .read_regular(&prepared.temporary_name, MAX_PUBLICATION_BYTES)
            .map_err(|failure| failure.id())?
            .is_some_and(|current| {
                current.bytes == prepared.bytes
                    && current.identity.device == created.device
                    && current.identity.inode == created.inode
                    && current.identity.links == 1
            })
        {
            return Err(HostEffectExecutorErrorId::RenameRace);
        }
        let target = CString::new(prepared.target_name.as_str())
            .map_err(|_| HostEffectExecutorErrorId::UnsafeObject)?;
        if rename_noreplace(self.anchor.directory.as_raw_fd(), &temporary, &target) != 0 {
            return Err(match last_errno() {
                Some(libc::EEXIST) | Some(libc::ENOENT) => HostEffectExecutorErrorId::RenameRace,
                _ => HostEffectExecutorErrorId::Io,
            });
        }
        let committed = self
            .read_regular(&prepared.target_name, MAX_PUBLICATION_BYTES)
            .map_err(|failure| failure.id())?
            .ok_or(HostEffectExecutorErrorId::RenameRace)?;
        if committed.bytes != prepared.bytes
            || !committed.identity.same_after_rename(created)
            || self
                .observe_object(&prepared.temporary_name, 1, false)
                .map_err(|failure| failure.id())?
                .kind
                != PublicationObjectKind::Missing
        {
            return Err(HostEffectExecutorErrorId::RenameRace);
        }
        run_fault(
            FaultPoint::BeforeDirectoryFsync,
            &self.anchor.canonical_path,
        )
        .map_err(|_| HostEffectExecutorErrorId::SyncFailure)?;
        self.anchor
            .directory
            .sync_all()
            .map_err(|_| HostEffectExecutorErrorId::SyncFailure)?;
        let (observation, classification) = self
            .observe_inventory(InventoryObservationRequest {
                target_name: &prepared.target_name,
                expected_temporary_name: &prepared.temporary_name,
                expectation: prepared.expectation.clone(),
                ledger_head: current_ledger_head.clone(),
                acknowledgement: None,
                data_synced: true,
            })
            .map_err(|failure| failure.id())?;
        if classification.id()
            != super::super::lifecycle::PublicationClassificationId::CommittedBeforeAcknowledgement
        {
            return Err(HostEffectExecutorErrorId::FalsePassReceipt);
        }
        Ok(observation)
    }

    pub(super) fn acknowledge(
        &self,
        committed: &CommittedPublication,
        acknowledgement: PublicationAcknowledgementIdentity,
        ledger_head: super::super::HostEffectLedgerHead,
    ) -> Result<
        (PublicationInventoryObservation, PublicationClassification),
        HostEffectExecutorFailure,
    > {
        self.observe_inventory(InventoryObservationRequest {
            target_name: &committed.target_name,
            expected_temporary_name: &committed.temporary_name,
            expectation: committed.expectation.clone(),
            ledger_head,
            acknowledgement: Some(acknowledgement),
            data_synced: true,
        })
    }

    pub(super) fn reobserve_committed(
        &self,
        committed: &CommittedPublication,
        ledger_head: super::super::HostEffectLedgerHead,
    ) -> Result<
        (PublicationInventoryObservation, PublicationClassification),
        HostEffectExecutorFailure,
    > {
        self.observe_inventory(InventoryObservationRequest {
            target_name: &committed.target_name,
            expected_temporary_name: &committed.temporary_name,
            expectation: committed.expectation.clone(),
            ledger_head,
            acknowledgement: None,
            data_synced: true,
        })
    }
}
