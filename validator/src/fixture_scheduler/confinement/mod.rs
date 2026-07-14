mod backend;
mod policy;

pub(crate) use backend::ConfinementPlan;
pub use policy::ConfinementPolicy;
#[cfg(test)]
pub use policy::NetworkIsolation;
