#[cfg(test)]
const RECOVERY_AUTHORIZATION_SCHEMA: &str =
    "harness-ultragoal.host-effect-recovery-authorization.v2";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PublicationObjectKind {
    Missing,
    Regular,
    Symlink,
    Directory,
    Fifo,
    Socket,
    Device,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PublicationObjectObservation {
    name: String,
    kind: PublicationObjectKind,
    byte_length: u64,
    mode: u32,
    hard_links: u64,
    content_sha256: Option<String>,
    object_generation: u64,
    data_synced: bool,
}

pub(in crate::distribution::host_effect) struct PublicationObjectObservationRequest {
    pub name: String,
    pub kind: PublicationObjectKind,
    pub byte_length: u64,
    pub mode: u32,
    pub hard_links: u64,
    pub content_sha256: Option<String>,
    pub object_generation: u64,
    pub data_synced: bool,
}

impl PublicationObjectObservation {
    pub(in crate::distribution::host_effect) fn new(
        request: PublicationObjectObservationRequest,
    ) -> Result<Self, SupportedHostLifecycleError> {
        let PublicationObjectObservationRequest {
            name,
            kind,
            byte_length,
            mode,
            hard_links,
            content_sha256,
            object_generation,
            data_synced,
        } = request;
        if !valid_object_name(&name)
            || object_generation == 0
            || content_sha256
                .as_deref()
                .is_some_and(|value| !is_digest(value))
            || (kind == PublicationObjectKind::Missing
                && (byte_length != 0
                    || mode != 0
                    || hard_links != 0
                    || content_sha256.is_some()
                    || data_synced))
            || (kind == PublicationObjectKind::Regular
                && (hard_links != 1 || mode & 0o170000 != 0o100000))
            || (kind != PublicationObjectKind::Regular && data_synced)
            || (kind != PublicationObjectKind::Regular && content_sha256.is_some())
        {
            return Err(recovery_unsafe());
        }
        Ok(Self {
            name,
            kind,
            byte_length,
            mode,
            hard_links,
            content_sha256,
            object_generation,
            data_synced,
        })
    }

    pub(in crate::distribution::host_effect) fn missing(
        name: String,
        object_generation: u64,
    ) -> Result<Self, SupportedHostLifecycleError> {
        Self::new(PublicationObjectObservationRequest {
            name,
            kind: PublicationObjectKind::Missing,
            byte_length: 0,
            mode: 0,
            hard_links: 0,
            content_sha256: None,
            object_generation,
            data_synced: false,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(in crate::distribution::host_effect) struct ExpectedPublicationObjectIdentity {
    name: String,
    kind: PublicationObjectKind,
    byte_length: u64,
    mode: u32,
    hard_links: u64,
    content_sha256: Option<String>,
    data_synced: bool,
}

pub(in crate::distribution::host_effect) struct ExpectedRegularPublicationObject {
    pub name: String,
    pub byte_length: u64,
    pub mode: u32,
    pub hard_links: u64,
    pub content_sha256: String,
    pub data_synced: bool,
}

impl ExpectedPublicationObjectIdentity {
    pub(in crate::distribution::host_effect) fn missing(
        name: String,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if !valid_object_name(&name) {
            return Err(recovery_unsafe());
        }
        Ok(Self {
            name,
            kind: PublicationObjectKind::Missing,
            byte_length: 0,
            mode: 0,
            hard_links: 0,
            content_sha256: None,
            data_synced: false,
        })
    }

    pub(in crate::distribution::host_effect) fn regular(
        request: ExpectedRegularPublicationObject,
    ) -> Result<Self, SupportedHostLifecycleError> {
        let ExpectedRegularPublicationObject {
            name,
            byte_length,
            mode,
            hard_links,
            content_sha256,
            data_synced,
        } = request;
        if !valid_object_name(&name)
            || mode & 0o170000 != 0o100000
            || hard_links != 1
            || !is_digest(&content_sha256)
        {
            return Err(recovery_unsafe());
        }
        Ok(Self {
            name,
            kind: PublicationObjectKind::Regular,
            byte_length,
            mode,
            hard_links,
            content_sha256: Some(content_sha256),
            data_synced,
        })
    }

    fn matches(&self, observed: &PublicationObjectObservation) -> bool {
        self.name == observed.name
            && self.kind == observed.kind
            && self.byte_length == observed.byte_length
            && self.mode == observed.mode
            && self.hard_links == observed.hard_links
            && self.content_sha256 == observed.content_sha256
            && self.data_synced == observed.data_synced
    }

    fn same_renamed_object(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.byte_length == other.byte_length
            && self.mode == other.mode
            && self.hard_links == other.hard_links
            && self.content_sha256 == other.content_sha256
            && self.data_synced == other.data_synced
    }

    fn safe_temporary_metadata_matches(&self, observed: &PublicationObjectObservation) -> bool {
        self.name == observed.name
            && self.kind == observed.kind
            && self.mode == observed.mode
            && self.hard_links == observed.hard_links
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(in crate::distribution::host_effect) struct PublicationExpectation {
    effect_identity_sha256: String,
    prior: ExpectedPublicationObjectIdentity,
    next: ExpectedPublicationObjectIdentity,
    temporary: ExpectedPublicationObjectIdentity,
    publication_identity_sha256: String,
}
