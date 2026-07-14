const TREE_ENTRY_LIMIT: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TreeObjectKind {
    RegularFile,
    Symlink,
    Special,
}

#[derive(Clone, Eq, PartialEq)]
pub struct TreeObject {
    path: String,
    kind: TreeObjectKind,
    mode: u32,
    link_count: u64,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for TreeObject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TreeObject")
            .field("path", &self.path)
            .field("kind", &self.kind)
            .field("mode", &self.mode)
            .field("link_count", &self.link_count)
            .field("byte_length", &self.bytes.len())
            .finish()
    }
}

impl TreeObject {
    pub fn regular(path: String, mode: u32, bytes: Vec<u8>) -> Self {
        Self {
            path,
            kind: TreeObjectKind::RegularFile,
            mode,
            link_count: 1,
            bytes,
        }
    }
    pub fn adversarial(
        path: String,
        kind: TreeObjectKind,
        mode: u32,
        link_count: u64,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            path,
            kind,
            mode,
            link_count,
            bytes,
        }
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub const fn mode(&self) -> u32 {
        self.mode
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpectedTree {
    Absent,
    ExactDigest(String),
}

pub trait MaterializeEffects {
    fn read_tree(
        &mut self,
        maximum_entries: usize,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<TreeObject>>, ()>;

    fn compare_exchange_tree(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[TreeObject]>,
    ) -> Result<bool, ()>;
}

#[derive(Debug)]
pub struct MaterializeTransaction {
    installed_tree_sha256: String,
    previous: Option<Vec<TreeObject>>,
}

impl MaterializeTransaction {
    pub fn tree_sha256(&self) -> &str {
        &self.installed_tree_sha256
    }
}

pub fn materialize_package(
    plan: &PackagePlan,
    expected: &ExpectedTree,
    effects: &mut impl MaterializeEffects,
) -> Result<MaterializeTransaction, DistributionError> {
    validate_expected(expected)?;
    let previous = read(effects)?;
    let previous_sha256 = previous.as_deref().map(tree_sha256).transpose()?;
    if !expected_matches(expected, previous_sha256.as_deref()) {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    let replacement = planned_tree(plan);
    match effects.compare_exchange_tree(previous_sha256.as_deref(), Some(&replacement)) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(()) => return Err(error(DistributionErrorId::EffectFailed)),
    }
    let verification = read(effects).and_then(|observed| {
        let observed = observed.ok_or_else(|| error(DistributionErrorId::ArchiveMismatch))?;
        reconcile(plan, &observed)?;
        Ok(observed)
    });
    if let Err(failure) = verification {
        restore(effects, plan.source_tree_sha256(), previous.as_deref())?;
        return Err(failure);
    }
    Ok(MaterializeTransaction {
        installed_tree_sha256: plan.source_tree_sha256().to_owned(),
        previous,
    })
}

pub fn rollback_materialization(
    transaction: MaterializeTransaction,
    effects: &mut impl MaterializeEffects,
) -> Result<(), DistributionError> {
    restore(
        effects,
        &transaction.installed_tree_sha256,
        transaction.previous.as_deref(),
    )
}

pub fn reconcile(plan: &PackagePlan, observed: &[TreeObject]) -> Result<(), DistributionError> {
    validate_tree(observed)?;
    if observed.len() != plan.entries.len() {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    for (actual, expected) in observed.iter().zip(&plan.entries) {
        if actual.path != expected.path
            || actual.mode != expected.mode
            || actual.bytes.len() as u64 != expected.byte_length()
            || sha256(&actual.bytes) != expected.sha256
        {
            return Err(error(DistributionErrorId::ArchiveMismatch));
        }
    }
    Ok(())
}

pub fn tree_sha256(objects: &[TreeObject]) -> Result<String, DistributionError> {
    validate_tree(objects)?;
    #[derive(Serialize)]
    struct Row<'a> {
        path: &'a str,
        mode: u32,
        sha256: String,
        byte_length: u64,
    }
    let rows = objects
        .iter()
        .map(|row| Row {
            path: &row.path,
            mode: row.mode,
            sha256: sha256(&row.bytes),
            byte_length: row.bytes.len() as u64,
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&rows)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| error(DistributionErrorId::InvalidSpec))
}

fn planned_tree(plan: &PackagePlan) -> Vec<TreeObject> {
    plan.entries
        .iter()
        .map(|row| TreeObject::regular(row.path.clone(), row.mode, row.bytes.clone()))
        .collect()
}
