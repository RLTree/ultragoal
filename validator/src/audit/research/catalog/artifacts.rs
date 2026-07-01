use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone)]
pub(in crate::audit::research) struct SourceArtifact {
    pub(in crate::audit::research) digest: String,
    pub(in crate::audit::research) method: String,
    pub(in crate::audit::research) corpus_path: String,
    pub(in crate::audit::research) corpus_digest: String,
}

pub(in crate::audit::research) fn source_artifacts(
    cards: &Value,
) -> BTreeMap<String, SourceArtifact> {
    cards
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let id = text(row, "source_id");
            let digest = text(row, "source_artifact_digest");
            let method = text(row, "source_artifact_method");
            let corpus_path = text(row, "source_corpus_path");
            let corpus_digest = text(row, "source_corpus_digest");
            (!id.is_empty()).then_some((
                id,
                SourceArtifact {
                    digest,
                    method,
                    corpus_path,
                    corpus_digest,
                },
            ))
        })
        .collect()
}

fn text(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
