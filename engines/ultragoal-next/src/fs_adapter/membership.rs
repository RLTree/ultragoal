use super::*;

/// An immutable directory identity baseline derived from a complete initial listing.
/// The contained listing is scoped to the recorded limits and basename exclusions.
#[derive(Debug)]
pub struct MembershipCheckpoint {
    listing: Listing,
    root_path: PathBuf,
    limits: Limits,
}

impl MembershipCheckpoint {
    pub fn listing(&self) -> &Listing {
        &self.listing
    }
}

/// This observation contains no file-content claim and no atomic-snapshot claim.
#[derive(Debug, Eq, PartialEq)]
pub enum MembershipObservation {
    DirectoryIdentitiesUnchanged { checked_directories: usize },
}

#[derive(Debug)]
pub enum MembershipFallback {
    InitialListingIncomplete(Box<Listing>),
    ScopeChanged,
    RootChanged,
    DirectoryChanged { path: Vec<u8> },
    Unavailable { path: Vec<u8>, errno: i32 },
}

impl Root {
    pub fn membership_checkpoint(
        &self,
        limits: &Limits,
    ) -> Result<MembershipCheckpoint, MembershipFallback> {
        self.membership_checkpoint_excluding(limits, &[])
    }

    pub fn membership_checkpoint_excluding(
        &self,
        limits: &Limits,
        excluded_basenames: &[Vec<u8>],
    ) -> Result<MembershipCheckpoint, MembershipFallback> {
        let listing = self.enumerate_excluding(limits, excluded_basenames);
        if !listing.complete || !listing.issues.is_empty() || listing.root_identity.is_none() {
            return Err(MembershipFallback::InitialListingIncomplete(Box::new(
                listing,
            )));
        }
        let checkpoint = MembershipCheckpoint {
            listing,
            root_path: self.path.clone(),
            limits: limits.bounded(),
        };
        // The scan checks each directory as it visits it. Recheck all directories
        // together before admitting the baseline: an earlier directory may change
        // while a later subtree is being scanned.
        self.revalidate_membership(&checkpoint, limits, excluded_basenames)?;
        Ok(checkpoint)
    }

    pub fn revalidate_membership(
        &self,
        checkpoint: &MembershipCheckpoint,
        limits: &Limits,
        excluded_basenames: &[Vec<u8>],
    ) -> Result<MembershipObservation, MembershipFallback> {
        if self.path != checkpoint.root_path
            || limits.bounded() != checkpoint.limits
            || !valid_exclusions(excluded_basenames)
        {
            return Err(MembershipFallback::ScopeChanged);
        }
        let mut excluded = excluded_basenames.to_vec();
        excluded.sort();
        excluded.dedup();
        if excluded != checkpoint.listing.excluded_basenames {
            return Err(MembershipFallback::ScopeChanged);
        }
        let expected_root = checkpoint
            .listing
            .root_identity
            .expect("only complete listings can create checkpoints");
        self.check_root(expected_root)?;
        let mut checked = 1;
        for member in &checkpoint.listing.members {
            if member.kind != Kind::Directory {
                continue;
            }
            let directory = self.walk_directory(&member.path).map_err(|error| {
                MembershipFallback::Unavailable {
                    path: member.path.clone(),
                    errno: code(error),
                }
            })?;
            let identity =
                Identity::of(&directory).map_err(|error| MembershipFallback::Unavailable {
                    path: member.path.clone(),
                    errno: code(error),
                })?;
            if identity != member.identity {
                return Err(MembershipFallback::DirectoryChanged {
                    path: member.path.clone(),
                });
            }
            checked += 1;
        }
        self.check_root(expected_root)?;
        Ok(MembershipObservation::DirectoryIdentitiesUnchanged {
            checked_directories: checked,
        })
    }

    fn check_root(&self, expected: Identity) -> Result<(), MembershipFallback> {
        let held =
            Identity::of(&self.directory).map_err(|error| MembershipFallback::Unavailable {
                path: Vec::new(),
                errno: code(error),
            })?;
        let reachable =
            os::open_root(&self.path).map_err(|error| MembershipFallback::Unavailable {
                path: Vec::new(),
                errno: code(error),
            })?;
        let current =
            Identity::of(&reachable).map_err(|error| MembershipFallback::Unavailable {
                path: Vec::new(),
                errno: code(error),
            })?;
        if held != expected || current != expected {
            return Err(MembershipFallback::RootChanged);
        }
        Ok(())
    }
}
