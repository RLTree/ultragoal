use super::{read, root};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

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
            let bytes = std::fs::read(root().join(&path)).expect("agent descriptor");
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

fn validate_descriptor_bindings(
    entries: &[(String, Vec<u8>)],
    request_bytes: &[u8],
) -> Result<(), DescriptorError> {
    let request: serde_json::Value =
        serde_json::from_slice(request_bytes).map_err(|_| DescriptorError::StaleBinding)?;
    let bindings = request["descriptor_bindings"]
        .as_array()
        .ok_or(DescriptorError::StaleBinding)?;
    if bindings.len() != AGENTS.len() {
        return Err(DescriptorError::StaleBinding);
    }
    for (path, bytes) in entries {
        let binding = bindings
            .iter()
            .find(|row| row["path"].as_str() == Some(path))
            .ok_or(DescriptorError::StaleBinding)?;
        let expected = AGENTS
            .iter()
            .find(|agent| agent.path() == *path)
            .ok_or(DescriptorError::UnknownPath)?;
        let lines = bytes.iter().filter(|byte| **byte == b'\n').count() as u64;
        let digest = sha256(bytes);
        if binding["name"].as_str() != Some(expected.name)
            || binding["sha256"].as_str() != Some(digest.as_str())
            || binding["byte_length"].as_u64() != Some(bytes.len() as u64)
            || binding["line_count"].as_u64() != Some(lines)
        {
            return Err(DescriptorError::StaleBinding);
        }
    }
    Ok(())
}

fn validate_manifest_membership(
    package_bytes: &[u8],
    plugin_bytes: &[u8],
) -> Result<(), DescriptorError> {
    let package: serde_json::Value =
        serde_json::from_slice(package_bytes).map_err(|_| DescriptorError::PackageMembership)?;
    let rows = package["agents"]
        .as_array()
        .ok_or(DescriptorError::PackageMembership)?;
    let actual = rows
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap_or_default().to_owned(),
                row["path"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect::<BTreeSet<_>>();
    let expected = AGENTS
        .iter()
        .map(|agent| (agent.name.to_owned(), agent.path()))
        .collect::<BTreeSet<_>>();
    if rows.len() != AGENTS.len() || actual != expected {
        return Err(DescriptorError::PackageMembership);
    }
    let plugin: serde_json::Value =
        serde_json::from_slice(plugin_bytes).map_err(|_| DescriptorError::PackageMembership)?;
    if plugin["name"] != "harness-ultragoal"
        || plugin["skills"] != "./skills/"
        || plugin.get("agents").is_some()
    {
        return Err(DescriptorError::PackageMembership);
    }
    Ok(())
}

fn validate_descriptor_bundle(
    entries: &[(String, Vec<u8>)],
    request_bytes: &[u8],
    package_bytes: &[u8],
    plugin_bytes: &[u8],
) -> Result<(), DescriptorError> {
    validate_descriptor_set(entries)?;
    validate_descriptor_bindings(entries, request_bytes)?;
    validate_manifest_membership(package_bytes, plugin_bytes)
}

fn canonical_source() -> String {
    let mut source = [
        read("README.md"),
        read("docs/plugin-resource-map.md"),
        read("docs/install-and-visibility.md"),
    ]
    .join("\n");
    for skill in SKILLS {
        source.push_str(&read(&format!("skills/{skill}/SKILL.md")));
    }
    source
}

#[test]
fn exactly_eight_canonical_skills_have_valid_unique_identity() {
    let mut names = BTreeSet::new();
    for expected in SKILLS {
        let source = read(&format!("skills/{expected}/SKILL.md"));
        assert!(source.starts_with("---\nname: "));
        let name = source
            .lines()
            .find_map(|line| line.strip_prefix("name: "))
            .expect("skill name");
        assert_eq!(name, *expected);
        assert!(
            source
                .lines()
                .any(|line| line.starts_with("description: \""))
        );
        assert!(names.insert(name.to_owned()), "duplicate skill {name}");
    }
    assert_eq!(names.len(), 8);
}

#[test]
fn one_front_door_routes_to_each_specialized_skill_once() {
    let front = read("skills/harness-ultragoal/SKILL.md");
    for route in SKILLS.iter().skip(1) {
        let reference = format!("$harness-ultragoal:{route}");
        assert_eq!(front.matches(&reference).count(), 1, "route {route}");
    }
    for route in SKILLS.iter().skip(1) {
        let source = read(&format!("skills/{route}/SKILL.md"));
        assert!(!source.contains("only front door"));
    }
}

#[test]
fn fresh_operator_sources_do_not_route_to_global_agents_or_old_product_entries() {
    let source = canonical_source();
    for prohibited in [
        "custom-agents/",
        "~/.codex/agents/",
        "$harness-ultragoal:fit-repo",
        "$harness-ultragoal:ultragoal",
        "$harness-ultragoal:harness-engineering",
        "$harness-ultragoal:execplan-lane",
        "$harness-ultragoal:proof-gate",
        "proposal bundle",
    ] {
        assert!(
            !source.contains(prohibited),
            "prohibited fresh route: {prohibited}"
        );
    }
}

#[test]
fn dirty_tree_and_zero_write_rules_are_explicit() {
    let routine = read("skills/routine-work/SKILL.md");
    for token in [
        "modified",
        "staged",
        "untracked",
        "worktree",
        "Never stash, clean, reset, checkout, stage, commit",
        "unexpected mutation",
    ] {
        assert!(routine.contains(token), "missing dirty-tree rule {token}");
    }
    let source = canonical_source();
    assert!(source.contains("zero hidden writes"));
    assert!(source.contains("access metadata"));
}

#[test]
fn six_project_agents_are_exact_and_read_only() {
    let directory = root().join(".codex/agents");
    let found = std::fs::read_dir(directory)
        .expect("agent directory")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .path()
                .file_stem()
                .and_then(|value| value.to_str())
                .map(str::to_owned)
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        found,
        AGENTS.iter().map(|agent| agent.name.to_owned()).collect()
    );
    for agent in AGENTS {
        let source = read(&agent.path());
        assert!(source.contains(&format!("name = \"{}\"", agent.name)));
        assert!(source.contains("sandbox_mode = \"read-only\""));
    }
    let entries = live_descriptors();
    validate_descriptor_bundle(
        &entries,
        read("fixtures/plugin-product/root-wiring-request.json").as_bytes(),
        read("plugin-manifest-draft.json").as_bytes(),
        read(".codex-plugin/plugin.json").as_bytes(),
    )
    .expect("semantic descriptor bundle");
}

#[test]
fn descriptor_unknown_missing_duplicate_semantic_authority_and_stale_controls_fail() {
    let live = live_descriptors();

    let mut unknown = live.clone();
    unknown.push((
        ".codex/agents/unknown-reviewer.toml".to_owned(),
        live[0].1.clone(),
    ));
    assert_eq!(
        validate_descriptor_set(&unknown),
        Err(DescriptorError::UnknownPath)
    );

    let mut missing = live.clone();
    missing.pop();
    assert_eq!(
        validate_descriptor_set(&missing),
        Err(DescriptorError::MissingDescriptor)
    );

    let mut duplicate = live.clone();
    let security = duplicate
        .iter_mut()
        .find(|(path, _)| path.ends_with("security-reviewer.toml"))
        .expect("security descriptor");
    security.1 = String::from_utf8(security.1.clone())
        .unwrap()
        .replace("name = \"security-reviewer\"", "name = \"claim-falsifier\"")
        .into_bytes();
    assert_eq!(
        validate_descriptor_set(&duplicate),
        Err(DescriptorError::DuplicateAgent)
    );

    let mut semantic = live.clone();
    semantic[0].1 = String::from_utf8(semantic[0].1.clone())
        .unwrap()
        .replace(
            AGENTS[0].description,
            "Read-only but semantically substituted description.",
        )
        .into_bytes();
    assert_eq!(
        validate_descriptor_set(&semantic),
        Err(DescriptorError::SemanticMismatch)
    );

    let mut escalated = live.clone();
    escalated[0].1 = String::from_utf8(escalated[0].1.clone())
        .unwrap()
        .replace(
            "sandbox_mode = \"read-only\"",
            "sandbox_mode = \"workspace-write\"",
        )
        .into_bytes();
    assert_eq!(
        validate_descriptor_set(&escalated),
        Err(DescriptorError::AuthorityEscalation)
    );

    let mut request: serde_json::Value =
        serde_json::from_str(&read("fixtures/plugin-product/root-wiring-request.json")).unwrap();
    request["descriptor_bindings"][0]["sha256"] =
        serde_json::Value::String(format!("sha256:{}", "0".repeat(64)));
    assert_eq!(
        validate_descriptor_bindings(&live, &serde_json::to_vec(&request).unwrap()),
        Err(DescriptorError::StaleBinding)
    );
}

#[test]
fn package_membership_missing_duplicate_and_unknown_rows_fail_closed() {
    let plugin = read(".codex-plugin/plugin.json");
    let package = read("plugin-manifest-draft.json");
    validate_manifest_membership(package.as_bytes(), plugin.as_bytes()).unwrap();

    let mut missing: serde_json::Value = serde_json::from_str(&package).unwrap();
    missing["agents"].as_array_mut().unwrap().pop();
    assert_eq!(
        validate_manifest_membership(&serde_json::to_vec(&missing).unwrap(), plugin.as_bytes()),
        Err(DescriptorError::PackageMembership)
    );

    let mut duplicate: serde_json::Value = serde_json::from_str(&package).unwrap();
    let row = duplicate["agents"][0].clone();
    duplicate["agents"].as_array_mut().unwrap().push(row);
    assert_eq!(
        validate_manifest_membership(&serde_json::to_vec(&duplicate).unwrap(), plugin.as_bytes()),
        Err(DescriptorError::PackageMembership)
    );

    let mut unknown: serde_json::Value = serde_json::from_str(&package).unwrap();
    unknown["agents"][0]["path"] =
        serde_json::Value::String(".codex/agents/unknown.toml".to_owned());
    assert_eq!(
        validate_manifest_membership(&serde_json::to_vec(&unknown).unwrap(), plugin.as_bytes()),
        Err(DescriptorError::PackageMembership)
    );
}

#[test]
fn canonical_commands_match_the_typed_successor_catalog() {
    let catalog = read("validator/src/cli/successor/catalog.rs");
    for group in [
        "InspectTarget::Capabilities",
        "FitAction::Inspect",
        "FitAction::Plan",
        "FitAction::Apply",
        "FitAction::Verify",
        "CheckProfile::Routine",
        "CheckProfile::Strict",
        "SuccessorCommand::Diagnose",
        "SuccessorCommand::Prove",
        "ObserveAction::Query",
        "ObserveAction::Export",
        "EvalAction::Audit",
        "MigrateAction::Plan",
        "MigrateAction::Verify",
        "MigrateAction::Retire",
    ] {
        assert!(catalog.contains(group), "missing typed command {group}");
    }
}

#[test]
fn source_install_discovery_runtime_and_journey_ceilings_stay_separate() {
    let install = read("docs/install-and-visibility.md");
    for layer in [
        "**Source:**",
        "**Package:**",
        "**Marketplace:**",
        "**Install:**",
        "**Cache:**",
        "**App registry and Plugins UI:**",
        "**Discovery:**",
        "**Runtime:**",
        "**Product journey:**",
    ] {
        assert!(install.contains(layer), "missing proof layer {layer}");
    }
    assert!(install.contains("A successful lower layer does not raise a higher claim."));
}

#[test]
fn root_wiring_request_is_exact_but_non_authoritative() {
    let request = read("fixtures/plugin-product/root-wiring-request.json");
    assert!(request.contains("\"authoritative\": false"));
    assert!(request.contains("\"value\": \"0.0.12\""));
    assert!(request.contains("./plugins/harness-ultragoal"));
    assert!(request.contains("path_absent"));
    let skills = request
        .split_once("\"pointer\": \"/skills\"")
        .expect("skills operation")
        .1
        .split_once("\"pointer\": \"/purpose\"")
        .expect("purpose operation")
        .0;
    for skill in SKILLS {
        assert_eq!(
            skills.matches(&format!("\"name\": \"{skill}\"")).count(),
            1,
            "root request skill {skill}"
        );
    }
    for agent in AGENTS {
        assert!(request.contains(&format!("\"{}\"", agent.name)));
        assert!(request.contains(&agent.path()));
    }
    assert_eq!(
        read(".codex-plugin/plugin.json").matches("0.0.11").count(),
        1
    );
    assert!(!root().join(".agents/plugins/marketplace.json").exists());
}
