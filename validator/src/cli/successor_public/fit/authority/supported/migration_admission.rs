use super::super::{
    RepositoryFitAuthorityMigrationAdmission, RepositoryFitAuthorityQuarantinePlan,
};
use super::state_open::open_host_state_base;
use super::*;
use sha2::{Digest, Sha256};

const MAX_PENDING_ENTRIES: usize = 1024;

pub(crate) fn assess_migration_admission(
    home: &Path,
) -> Result<Option<RepositoryFitAuthorityMigrationAdmission>, HostFailure> {
    let Some(current) = open_host_state_base(home)? else {
        return Ok(None);
    };
    let Some(parent) = current.open_child_optional(FIT_STATE_COMPONENTS[0], true)? else {
        return Ok(None);
    };
    let Some(current) = parent.open_child_optional(FIT_STATE_COMPONENTS[1], true)? else {
        return Ok(None);
    };
    assess_migration_admission_for_owner(current, parent.identity)
}

pub(super) fn assess_migration_admission_for_owner(
    current: AnchoredDirectory,
    parent_identity: FileIdentity,
) -> Result<Option<RepositoryFitAuthorityMigrationAdmission>, HostFailure> {
    let Some(authority) = current.open_child_optional(AUTHORITY_DIRECTORY, true)? else {
        return Ok(None);
    };
    let Some(pending) = current.open_child_optional(PENDING_DIRECTORY, true)? else {
        return Err(HostFailure::Invalid);
    };

    let store_id = digest(STORE_DOMAIN);
    let drift = match FileRepositoryFitLedger::assess_device_drift(&authority.path, &store_id) {
        Ok(Some(drift)) => drift,
        Ok(None) => return Ok(None),
        Err(_) => return Err(HostFailure::Invalid),
    };
    let pending_inventory = pending_inventory(&pending)?;
    current.verify(true)?;
    authority.verify(true)?;
    pending.verify(true)?;

    let source_inventory_sha256 = digest(
        &serde_json::to_vec(&(
            "repository-fit-authority-migration-source-inventory-v1",
            directory_identity_row(parent_identity),
            identity_row(current.identity),
            identity_row(authority.identity),
            identity_row(pending.identity),
            &drift.authority_inventory_sha256,
            &pending_inventory.inventory_sha256,
        ))
        .map_err(|_| HostFailure::Invalid)?,
    );
    let eligible =
        drift.nonterminal_reservation_count == 0 && pending_inventory.pending_envelope_count == 0;
    let plan = eligible.then(|| {
        quarantine_plan(
            &source_inventory_sha256,
            &drift.ledger_sha256,
            &drift.authority_id,
            &drift.head_sha256,
        )
    });
    let (status, next_action) = if drift.nonterminal_reservation_count != 0 {
        (
            "nonterminal_authority_present",
            "preserve the complete state owner and reconcile the existing reservation before any migration planning",
        )
    } else if pending_inventory.pending_envelope_count != 0 {
        (
            "pending_recovery_present",
            "preserve the complete state owner and service the exact pending recovery before any migration planning",
        )
    } else {
        (
            "device_identity_changed",
            "write this complete diagnosis projection to one owner-only regular file, then run ultragoal --json fit apply --plan <absolute-path> --accept-plan <plan_id>",
        )
    };
    Ok(Some(RepositoryFitAuthorityMigrationAdmission {
        status,
        authority_status: "authenticated_device_only_drift",
        generation: drift.generation,
        event_count: drift.event_count,
        reservation_count: drift.reservation_count,
        nonterminal_reservation_count: drift.nonterminal_reservation_count,
        pending_envelope_count: pending_inventory.pending_envelope_count,
        process_lock_count: pending_inventory.process_lock_count,
        stored_root: drift.stored_root,
        current_root: drift.current_root,
        stored_lock: drift.stored_lock,
        current_lock: drift.current_lock,
        quarantine_plan: plan,
        next_action,
    }))
}

fn quarantine_plan(
    source_inventory_sha256: &str,
    ledger_sha256: &str,
    authority_id: &str,
    head_sha256: &str,
) -> RepositoryFitAuthorityQuarantinePlan {
    let plan_id = digest(
        &serde_json::to_vec(&(
            "repository-fit-authority-quarantine-plan-v1",
            source_inventory_sha256,
            ledger_sha256,
            authority_id,
            head_sha256,
            "whole_owner_quarantine_then_fresh_bootstrap",
        ))
        .expect("fixed migration plan identity is serializable"),
    );
    let stem = plan_id
        .strip_prefix("sha256:")
        .expect("digest has a sha256 prefix")
        .to_owned();
    RepositoryFitAuthorityQuarantinePlan {
        plan_id,
        source_inventory_sha256: source_inventory_sha256.to_owned(),
        ledger_sha256: ledger_sha256.to_owned(),
        authority_id: authority_id.to_owned(),
        ledger_head_sha256: head_sha256.to_owned(),
        source_owner: "repository-fit",
        quarantine_owner: format!("repository-fit.quarantine.{}", &stem[..16]),
        target_format: "harness-ultragoal.repository-fit-authority-ledger.v3",
        strategy: "whole_owner_quarantine_then_fresh_bootstrap",
        apply_capability: "fit_apply_exact_quarantine_plan",
        operations: vec![
            "revalidate_complete_source_inventory_under_exclusive_parent_custody",
            "quarantine_complete_repository_fit_owner_without_rewriting_history",
            "fsync_parent_and_revalidate_quarantined_inventory",
            "initialize_fresh_repository_fit_owner_only_after_review",
            "retain_quarantine_until_same_surface_installed_journey_passes",
        ],
        rollback: "before fresh bootstrap, rename the exact unchanged quarantine owner back; after bootstrap, stop and require a separately reviewed reconciliation",
    }
}

struct PendingInventory {
    inventory_sha256: String,
    pending_envelope_count: usize,
    process_lock_count: usize,
}

fn pending_inventory(pending: &AnchoredDirectory) -> Result<PendingInventory, HostFailure> {
    pending.verify(true)?;
    // SAFETY: the directory descriptor is live; openat returns a new descriptor on success.
    let descriptor = unsafe {
        libc::openat(
            pending.file.as_raw_fd(),
            c".".as_ptr(),
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_CLOEXEC
                | libc::O_NOFOLLOW
                | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(HostFailure::Invalid);
    }
    // SAFETY: ownership of the new descriptor transfers to the directory stream.
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        // SAFETY: fdopendir failed, so this call still owns descriptor.
        unsafe { libc::close(descriptor) };
        return Err(HostFailure::Invalid);
    }
    let mut rows = Vec::new();
    let mut pending_envelope_count = 0;
    let mut process_lock_count = 0;
    let result = loop {
        // SAFETY: __error returns this thread's writable errno location.
        unsafe { *libc::__error() = 0 };
        // SAFETY: stream remains live until closed below.
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            // SAFETY: __error returns this thread's readable errno location.
            break if unsafe { *libc::__error() } == 0 {
                Ok(())
            } else {
                Err(HostFailure::Invalid)
            };
        }
        // SAFETY: readdir returned a NUL-terminated entry name.
        let bytes = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if matches!(bytes, b"." | b"..") {
            continue;
        }
        if rows.len() >= MAX_PENDING_ENTRIES {
            break Err(HostFailure::Invalid);
        }
        let name = std::str::from_utf8(bytes).map_err(|_| HostFailure::Invalid)?;
        let kind = pending_name_kind(name).ok_or(HostFailure::Invalid)?;
        let observed = stat_at(&pending.file, name)?.ok_or(HostFailure::Invalid)?;
        if !observed.safe_regular() || observed.device != pending.identity.device {
            break Err(HostFailure::Invalid);
        }
        let mut file = openat(
            &pending.file,
            name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        let opened = file
            .metadata()
            .map(|metadata| identity(&metadata))
            .map_err(|_| HostFailure::Invalid)?;
        if opened != observed {
            break Err(HostFailure::Invalid);
        }
        let mut contents = Vec::new();
        (&mut file)
            .take(MAX_PENDING_BYTES + 1)
            .read_to_end(&mut contents)
            .map_err(|_| HostFailure::Invalid)?;
        if contents.len() as u64 > MAX_PENDING_BYTES
            || file
                .metadata()
                .map(|metadata| identity(&metadata))
                .map_err(|_| HostFailure::Invalid)?
                != opened
            || stat_at(&pending.file, name)?.ok_or(HostFailure::Invalid)? != observed
        {
            break Err(HostFailure::Invalid);
        }
        match kind {
            "pending" => pending_envelope_count += 1,
            "lock" => process_lock_count += 1,
            _ => unreachable!(),
        }
        rows.push((
            name.to_owned(),
            kind,
            identity_row(observed),
            digest(&contents),
        ));
    };
    // SAFETY: stream owns descriptor and is closed exactly once here.
    if unsafe { libc::closedir(stream) } != 0 {
        return Err(HostFailure::Invalid);
    }
    result?;
    rows.sort();
    pending.verify(true)?;
    let inventory_sha256 = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&rows).map_err(|_| HostFailure::Invalid)?)
    );
    Ok(PendingInventory {
        inventory_sha256,
        pending_envelope_count,
        process_lock_count,
    })
}

fn pending_name_kind(name: &str) -> Option<&'static str> {
    let (stem, kind) = if let Some(stem) = name.strip_suffix(".pending.json") {
        (stem, "pending")
    } else if let Some(stem) = name.strip_suffix(".lock") {
        (stem, "lock")
    } else {
        return None;
    };
    (stem.len() == 64
        && stem
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()))
    .then_some(kind)
}

fn identity_row(identity: FileIdentity) -> (u64, u64, u32, u32, u64, u64, u32) {
    (
        identity.device,
        identity.inode,
        identity.uid,
        identity.mode,
        identity.links,
        identity.size,
        identity.kind,
    )
}

fn directory_identity_row(identity: FileIdentity) -> (u64, u64, u32, u32, u32) {
    (
        identity.device,
        identity.inode,
        identity.uid,
        identity.mode,
        identity.kind,
    )
}
