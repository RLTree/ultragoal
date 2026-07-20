use crate::distribution::{InstallTransaction, ScopedFile};

#[derive(Clone, Copy)]
pub(super) enum Surface {
    Installed,
    Cache,
}

pub(super) enum Mutation {
    Installed {
        surface: Surface,
        transaction: Box<InstallTransaction>,
    },
    Removed {
        surface: Surface,
        file: ScopedFile,
        previous: Vec<u8>,
    },
}
