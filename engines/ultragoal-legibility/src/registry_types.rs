use crate::model::*;
#[derive(Debug)]
pub struct RegistryError(pub String);
impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl From<String> for RegistryError {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for RegistryError {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

// Private crate contract: arbitrary Value/unknown structs cannot enter the registry decoder.
pub trait RegistryPayload: serde::de::DeserializeOwned + sealed::Payload {}
impl<T: serde::de::DeserializeOwned + sealed::Payload> RegistryPayload for T {}
mod sealed {
    use super::*;
    pub trait Payload {}
    impl Payload for Registry {}
    impl Payload for Vec<SourceRow> {}
    impl Payload for Vec<DependencyRow> {}
    impl Payload for Vec<DependencyProfile> {}
    impl Payload for Vec<BoundaryRow> {}
    impl Payload for Vec<CommandRow> {}
    impl Payload for Vec<OutputRow> {}
}
