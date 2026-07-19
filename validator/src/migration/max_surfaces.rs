const MAX_SURFACES: usize = 16_384;
#[cfg(test)]
const MAX_ROUTES: usize = 4_096;
const MAX_REFS_PER_SURFACE: usize = 4_096;
const MAX_IDENTIFIER_BYTES: usize = 160;
const MAX_PATH_BYTES: usize = 768;
#[cfg(test)]
const MAX_AUTHORIZATION_TTL_MS: u64 = 10 * 60 * 1_000;
const REQUIRED_FALSE_PASS_CONTROLS: [&str; 5] = [
    "proof-artifact",
    "receipt-production",
    "score-only",
    "test-manipulation",
    "verbosity",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationError {
    code: &'static str,
}

impl MigrationError {
    pub(crate) fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for MigrationError {}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceFileKind {
    Regular,
    Semantic,
    Directory,
    Symlink,
    Special,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceStatus {
    Active,
    Candidate,
    Definition,
    ContextOnly,
    Retired,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InventorySurface {
    stable_id: String,
    kind: String,
    relative_path: String,
    digest_sha256: String,
    file_kind: SurfaceFileKind,
    link_count: u64,
    status: SurfaceStatus,
    active_readers: Vec<String>,
    active_writers: Vec<String>,
    public_routes: Vec<String>,
    generated_outputs: Vec<String>,
}

pub struct InventorySurfaceObservation {
    pub stable_id: String,
    pub kind: String,
    pub relative_path: String,
    pub digest_sha256: String,
    pub file_kind: SurfaceFileKind,
    pub link_count: u64,
    pub status: SurfaceStatus,
    pub active_readers: Vec<String>,
    pub active_writers: Vec<String>,
    pub public_routes: Vec<String>,
    pub generated_outputs: Vec<String>,
}

impl InventorySurface {
    pub fn observed(observation: InventorySurfaceObservation) -> Self {
        let InventorySurfaceObservation {
            stable_id,
            kind,
            relative_path,
            digest_sha256,
            file_kind,
            link_count,
            status,
            active_readers,
            active_writers,
            public_routes,
            generated_outputs,
        } = observation;
        Self {
            stable_id,
            kind,
            relative_path,
            digest_sha256,
            file_kind,
            link_count,
            status,
            active_readers: normalized(active_readers),
            active_writers: normalized(active_writers),
            public_routes: normalized(public_routes),
            generated_outputs: normalized(generated_outputs),
        }
    }

    pub fn stable_id(&self) -> &str {
        &self.stable_id
    }

    pub fn status(&self) -> SurfaceStatus {
        self.status
    }

    fn findings(&self) -> Vec<String> {
        let mut findings = Vec::new();
        if !valid_stable_identifier(&self.stable_id) || !valid_identifier(&self.kind) {
            findings.push("migration-surface-identity-invalid".to_owned());
        }
        if !safe_relative_path(&self.relative_path) {
            findings.push("migration-surface-path-unsafe".to_owned());
        }
        if !valid_sha256(&self.digest_sha256) {
            findings.push("migration-surface-digest-invalid".to_owned());
        }
        if !matches!(
            (self.file_kind, self.link_count),
            (SurfaceFileKind::Regular, 1) | (SurfaceFileKind::Semantic, 0)
        ) {
            findings.push("migration-surface-identity-invalid".to_owned());
        }
        for (label, values) in [
            ("reader", &self.active_readers),
            ("writer", &self.active_writers),
            ("route", &self.public_routes),
            ("generated", &self.generated_outputs),
        ] {
            if values.len() > MAX_REFS_PER_SURFACE
                || values.iter().any(|value| !safe_reference(value))
                || values.windows(2).any(|pair| pair[0] == pair[1])
            {
                findings.push(format!("migration-surface-{label}-set-invalid"));
            }
        }
        findings
    }

    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{:?}|{}|{:?}|{}|{}|{}|{}",
            self.stable_id,
            self.kind,
            self.relative_path,
            self.digest_sha256,
            self.file_kind,
            self.link_count,
            self.status,
            self.active_readers.join(","),
            self.active_writers.join(","),
            self.public_routes.join(","),
            self.generated_outputs.join(",")
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MigrationInventory {
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    surfaces: Vec<InventorySurface>,
    inventory_sha256: String,
}
