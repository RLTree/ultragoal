use super::*;

impl LoadedManifest {
    pub(crate) fn graph(&self) -> Result<ImpactGraph, ()> {
        let nodes = self
            .nodes
            .iter()
            .map(|node| {
                RunnerSpec::new(ROUTINE_RUNNER, None).and_then(|runner| {
                    CheckNode::new(
                        &node.node_id,
                        node.depends_on.clone(),
                        CheckClass::Routine,
                        runner,
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ())?;
        let routes = self
            .routes
            .iter()
            .map(|route| {
                let path = RepoPath::parse(&route.path)?;
                let matcher = match route.matcher {
                    MatcherKind::Exact => PathMatcher::Exact(path),
                    MatcherKind::Prefix => PathMatcher::Prefix(path),
                };
                PathRoute::new(
                    &route.row_id,
                    matcher,
                    route.node_ids.clone(),
                    route.requires_strict,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ())?;
        ImpactGraph::new(nodes, routes, Vec::new()).map_err(|_| ())
    }

    pub(crate) fn node(&self, node_id: &str) -> Option<&ManifestNode> {
        self.nodes.iter().find(|node| node.node_id == node_id)
    }

    pub(crate) fn selected_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![
            PathBuf::from(MANIFEST_PATH),
            PathBuf::from(&self.catalog.path),
        ];
        paths.extend(
            self.nodes
                .iter()
                .flat_map(|node| node.read_sources.iter().map(PathBuf::from)),
        );
        paths.sort();
        paths.dedup();
        paths
    }

    pub(crate) fn tool_names(&self) -> Vec<String> {
        vec![ROUTINE_RUNNER.to_owned()]
    }

    pub(crate) fn source_id(&self) -> String {
        #[derive(serde::Serialize)]
        struct Source<'a> {
            manifest_sha256: &'a str,
            manifest_byte_length: u64,
            catalog_sha256: &'a str,
            catalog_byte_length: u64,
        }
        let bytes = serde_json::to_vec(&Source {
            manifest_sha256: &self.source_sha256,
            manifest_byte_length: self.source_byte_length,
            catalog_sha256: &self.catalog.sha256,
            catalog_byte_length: self.catalog.byte_length,
        })
        .expect("fixed source binding serializes");
        digest(&bytes)
    }
}

impl ManifestNode {
    pub(crate) fn output_scope(&self) -> String {
        format!("target/routine/{}", self.node_id)
    }

    pub(crate) fn canonical_arguments(&self) -> Vec<String> {
        vec![
            "--json".to_owned(),
            "check".to_owned(),
            "routine".to_owned(),
        ]
    }
}

pub(crate) fn has_duplicate(values: &[String]) -> bool {
    values.windows(2).any(|pair| pair[0] == pair[1])
        || values
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect::<BTreeSet<_>>()
            .len()
            != values.len()
}

pub(crate) fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn selected_input_map(context: &LiveContext) -> BTreeMap<String, (String, u64)> {
    context
        .selected_inputs()
        .iter()
        .map(|input| {
            (
                input.relative_path.clone(),
                (format!("sha256:{}", input.sha256), input.byte_length),
            )
        })
        .collect()
}
