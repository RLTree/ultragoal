use crate::distribution::ConfinedRoot;
use crate::host_lifecycle::{
    DarwinHostDiagnosis, DarwinHostError, DarwinHostErrorId, DarwinHostSurface,
    DarwinHostTransactionAdapter, DarwinHostTransactionDisposition, DarwinTestControl,
    DarwinTestPoint,
};
use crate::transaction_fixture::{Fixture, MARKETPLACE};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Barrier, mpsc};

fn reopen(fixture: &Fixture) -> Result<DarwinHostTransactionAdapter, DarwinHostError> {
    DarwinHostTransactionAdapter::open(ConfinedRoot::open(&fixture.root).unwrap())
}

mod effect_boundary_authority {
    include!("recovery/effect_boundary_authority.rs");
}
mod effect_failure_recovery {
    include!("recovery/effect_failure_recovery.rs");
}
mod effect_interruption {
    include!("recovery/effect_interruption.rs");
}
mod lineage_publication_recovery {
    include!("recovery/lineage_publication_recovery.rs");
}
mod repeated_recovery {
    include!("recovery/repeated_recovery.rs");
}
mod reservation_races {
    include!("recovery/reservation_races.rs");
}
mod terminal_lineage_recovery {
    include!("recovery/terminal_lineage_recovery.rs");
}
