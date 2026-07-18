#[cfg(test)]
mod backend;
mod policy;

#[cfg(test)]
pub(crate) use backend::ConfinementPlan;
pub use policy::ConfinementPolicy;
#[cfg(test)]
pub use policy::NetworkIsolation;
