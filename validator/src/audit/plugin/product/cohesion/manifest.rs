use serde_json::Value;

#[derive(Clone, Debug)]
pub(crate) struct PluginCohesionManifest {
    pub(crate) schema: String,
    pub(crate) entrypoints: Vec<String>,
    pub(crate) edges: Vec<PluginFlowEdge>,
    pub(crate) flows: Vec<PluginFlowStage>,
    pub(crate) required_surfaces: Vec<String>,
    pub(crate) skills: Vec<String>,
    pub(crate) schemas: Vec<String>,
    pub(crate) templates: Vec<String>,
    pub(crate) custom_agents: Vec<String>,
    pub(crate) setup_scripts: Vec<String>,
    pub(crate) fixture_groups: Vec<String>,
    pub(crate) receipts: Vec<String>,
    pub(crate) validator_checks: Vec<String>,
    pub(crate) standards_rows: Vec<String>,
    pub(crate) package_cache_install_surfaces: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PluginFlowEdge {
    from: String,
    to: String,
}

#[derive(Clone, Debug)]
pub(crate) struct PluginFlowStage {
    pub(crate) id: String,
    pub(crate) completion_receipt: String,
    pub(crate) required_edges: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PackageManifestProjection {
    pub(crate) skills: Vec<String>,
    pub(crate) schemas: Vec<String>,
    pub(crate) authorable_templates: Vec<String>,
    pub(crate) agents: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct StandardsRowProjection {
    pub(crate) row_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PluginPromptProjection {
    default_prompt: String,
}

#[derive(Clone, Debug)]
pub(crate) struct PluginResourceMapProjection {
    text: String,
}

impl PluginCohesionManifest {
    pub(crate) fn from_value(value: &Value) -> Self {
        Self {
            schema: field(value, "schema"),
            entrypoints: strings(value, "entrypoints"),
            edges: value
                .get("edges")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(PluginFlowEdge::from_value)
                .collect(),
            flows: value
                .get("flows")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(PluginFlowStage::from_value)
                .collect(),
            required_surfaces: strings(value, "required_surfaces"),
            skills: strings(value, "skills"),
            schemas: strings(value, "schemas"),
            templates: strings(value, "templates"),
            custom_agents: strings(value, "custom_agents"),
            setup_scripts: strings(value, "setup_scripts"),
            fixture_groups: strings(value, "fixture_groups"),
            receipts: strings(value, "receipts"),
            validator_checks: strings(value, "validator_checks"),
            standards_rows: strings(value, "standards_rows"),
            package_cache_install_surfaces: strings(value, "package_cache_install_surfaces"),
        }
    }

    pub(crate) fn edge_keys(&self) -> Vec<String> {
        self.edges.iter().map(PluginFlowEdge::key).collect()
    }
}

impl PluginFlowEdge {
    fn from_value(value: &Value) -> Self {
        Self {
            from: field(value, "from"),
            to: field(value, "to"),
        }
    }

    fn key(&self) -> String {
        format!("{}->{}", self.from, self.to)
    }
}

impl PluginFlowStage {
    fn from_value(value: &Value) -> Self {
        Self {
            id: field(value, "id"),
            completion_receipt: field(value, "completion_receipt"),
            required_edges: strings(value, "required_edges"),
        }
    }
}

impl PackageManifestProjection {
    pub(crate) fn from_value(value: &Value) -> Self {
        Self {
            skills: manifest_paths(value, "skills"),
            schemas: manifest_paths(value, "schemas"),
            authorable_templates: manifest_paths(value, "authorable_templates"),
            agents: manifest_paths(value, "agents"),
        }
    }
}

impl StandardsRowProjection {
    pub(crate) fn from_value(value: &Value) -> Self {
        Self {
            row_ids: value
                .get("rows")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|row| row.get("id").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect(),
        }
    }
}

impl PluginPromptProjection {
    pub(crate) fn from_value(value: &Value) -> Self {
        Self {
            default_prompt: value
                .get("interface")
                .and_then(|item| item.get("defaultPrompt"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    pub(crate) fn mentions(&self, needle: &str) -> bool {
        self.default_prompt.contains(needle)
    }
}

impl PluginResourceMapProjection {
    pub(crate) fn from_text(text: String) -> Self {
        Self { text }
    }

    pub(crate) fn first_position(&self, needle: &str) -> Option<usize> {
        self.text.find(needle)
    }
}

fn manifest_paths(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.as_str().map(ToOwned::to_owned).or_else(|| {
                row.get("path")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
        })
        .collect()
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
