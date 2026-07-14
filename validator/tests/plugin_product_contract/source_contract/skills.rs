const SKILLS: &[&str] = &[
    "harness-ultragoal",
    "repository-fit",
    "routine-work",
    "diagnose-and-observe",
    "goal-run",
    "product-journey-review",
    "prove",
    "improve-and-maintain",
];

#[derive(Clone, Copy)]
struct AgentExpected {
    name: &'static str,
    description: &'static str,
    instructions: &'static str,
}

impl AgentExpected {
    fn path(self) -> String {
        format!(".codex/agents/{}.toml", self.name)
    }
}

const AGENTS: &[AgentExpected] = &[
    AgentExpected {
        name: "claim-falsifier",
        description: "Read-only falsification of claim prerequisites, evidence envelopes, guards, and ceilings.",
        instructions: "Attempt to disprove each proposed claim using its exact prerequisites, truth surface, guards, candidate identity, and false-pass controls. Report the highest defensible ceiling. Do not edit files, emit claim decisions, or declare readiness, release, or completion.",
    },
    AgentExpected {
        name: "orchestration-recovery-reviewer",
        description: "Read-only review of leases, dependency ordering, interruption, reconciliation, and recovery.",
        instructions: "Falsify ownership, dependency closure, lease isolation, worker-result reconciliation, interruption recovery, and honest-stop behavior. Do not edit files, coordinate external effects, or claim orchestration, readiness, release, or completion.",
    },
    AgentExpected {
        name: "product-journey-reviewer",
        description: "Read-only review of fresh setup, retrofit, routine use, diagnosis, recovery, and quality-in-use journeys.",
        instructions: "Falsify representative product journeys from a fresh operator perspective. Distinguish source, package, install, discovery, runtime, and product-behavior truth. Do not edit files, authorize effects, or substitute tests and receipts for the named behavior.",
    },
    AgentExpected {
        name: "repo-recon",
        description: "Read-only repository reconnaissance for live topology, commands, dependencies, and candidate identity.",
        instructions: "Inspect the live repository and report source-backed facts, uncertainties, and blockers. Do not edit files, run mutating commands, infer runtime metadata, or claim readiness or completion.",
    },
    AgentExpected {
        name: "research-verifier",
        description: "Read-only verification of current primary sources and capability claims.",
        instructions: "Verify time-sensitive product, platform, standard, and dependency claims against current primary sources. Separate documented support from live repository and runtime proof. Do not edit files or make product, readiness, release, or completion decisions.",
    },
    AgentExpected {
        name: "security-reviewer",
        description: "Read-only security review of trust boundaries, confinement, secrets, effects, and supply-chain behavior.",
        instructions: "Review attacker-controlled inputs, filesystem and process confinement, secret handling, effect authorization, provenance, and false-pass paths. Use non-destructive probes only, never inspect prohibited secret files, and do not edit or approve release state.",
    },
];

#[derive(Debug, Eq, PartialEq)]
enum DescriptorError {
    UnknownPath,
    MissingDescriptor,
    DuplicatePath,
    DuplicateAgent,
    InvalidToml,
    SemanticMismatch,
    AuthorityEscalation,
    StaleBinding,
    PackageMembership,
}

fn live_descriptors() -> Vec<(String, Vec<u8>)> {
    AGENTS
        .iter()
        .map(|agent| {
            let path = agent.path();
            let bytes = std::fs::read(root().join(&path))
                .unwrap_or_else(|error| panic!("agent descriptor {path} unavailable: {error}"));
            (path, bytes)
        })
        .collect()
}

fn text<'a>(value: &'a toml::Value, key: &str) -> Result<&'a str, DescriptorError> {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .ok_or(DescriptorError::SemanticMismatch)
}

fn validate_descriptor_set(entries: &[(String, Vec<u8>)]) -> Result<(), DescriptorError> {
    if entries.len() < AGENTS.len() {
        return Err(DescriptorError::MissingDescriptor);
    }
    let mut paths = BTreeSet::new();
    let mut names = BTreeSet::new();
    for (path, bytes) in entries {
        if !paths.insert(path.clone()) {
            return Err(DescriptorError::DuplicatePath);
        }
        let expected = AGENTS
            .iter()
            .find(|agent| agent.path() == *path)
            .ok_or(DescriptorError::UnknownPath)?;
        let source = std::str::from_utf8(bytes).map_err(|_| DescriptorError::InvalidToml)?;
        let value =
            toml::from_str::<toml::Value>(source).map_err(|_| DescriptorError::InvalidToml)?;
        let table = value.as_table().ok_or(DescriptorError::InvalidToml)?;
        let keys = table.keys().map(String::as_str).collect::<BTreeSet<_>>();
        if keys
            != BTreeSet::from([
                "name",
                "description",
                "developer_instructions",
                "sandbox_mode",
            ])
        {
            return Err(DescriptorError::SemanticMismatch);
        }
        let name = text(&value, "name")?;
        if !names.insert(name.to_owned()) {
            return Err(DescriptorError::DuplicateAgent);
        }
        let description = text(&value, "description")?;
        let instructions = text(&value, "developer_instructions")?;
        let sandbox = text(&value, "sandbox_mode")?;
        if sandbox != "read-only"
            || [description, instructions, sandbox]
                .join(" ")
                .to_ascii_lowercase()
                .contains("workspace-write")
        {
            return Err(DescriptorError::AuthorityEscalation);
        }
        let lower = [description, instructions].join(" ").to_ascii_lowercase();
        if [
            "~/.codex",
            "custom-agents",
            "marketplace",
            "install agent",
            "copy agent",
        ]
        .iter()
        .any(|token| lower.contains(token))
        {
            return Err(DescriptorError::AuthorityEscalation);
        }
        if name != expected.name
            || description != expected.description
            || instructions != expected.instructions
            || !instructions.to_ascii_lowercase().contains("do not")
        {
            return Err(DescriptorError::SemanticMismatch);
        }
    }
    if names != AGENTS.iter().map(|agent| agent.name.to_owned()).collect() {
        return Err(DescriptorError::MissingDescriptor);
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
