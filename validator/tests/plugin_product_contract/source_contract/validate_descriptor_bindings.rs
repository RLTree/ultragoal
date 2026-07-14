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
            .unwrap_or_else(|| panic!("skill name missing for {expected}"));
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
