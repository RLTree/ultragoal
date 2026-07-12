use super::artifact;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Clone)]
pub(crate) struct VerifiedRef {
    pub(crate) path: String,
    pub(crate) digest: String,
    pub(crate) value: Option<Value>,
}

pub(crate) fn verify(
    value: &Value,
    key: &str,
    expected: &[&str],
    root: Option<&Path>,
) -> Option<Vec<VerifiedRef>> {
    let declared = declarations(value, key, expected)?;
    let mut refs = Vec::with_capacity(declared.len());
    for (path, digest) in declared {
        let loaded = match root {
            Some(root) => Some(artifact::read(root, &path).ok()?),
            None => None,
        };
        if loaded
            .as_ref()
            .is_some_and(|artifact| artifact.digest != digest)
        {
            return None;
        }
        refs.push(VerifiedRef {
            path,
            digest,
            value: loaded.map(|artifact| artifact.value),
        });
    }
    Some(refs)
}

pub(crate) fn verify_loaded(
    value: &Value,
    key: &str,
    expected: &[&str],
    loaded: &[VerifiedRef],
) -> Option<Vec<VerifiedRef>> {
    let declared = declarations(value, key, expected)?;
    if loaded.len() != expected.len() {
        return None;
    }
    declared
        .into_iter()
        .map(|(path, digest)| {
            loaded
                .iter()
                .find(|row| row.path == path && row.digest == digest && row.value.is_some())
                .cloned()
        })
        .collect()
}

fn fields(object: &Map<String, Value>) -> Option<(&str, &str)> {
    if object.len() != 2 {
        return None;
    }
    Some((
        object.get("path")?.as_str()?,
        object.get("digest")?.as_str()?,
    ))
}

fn declarations(value: &Value, key: &str, expected: &[&str]) -> Option<Vec<(String, String)>> {
    let rows = value.get(key)?.as_array()?;
    if rows.len() != expected.len() {
        return None;
    }
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut declared = Vec::with_capacity(rows.len());
    for row in rows {
        let (path, digest) = fields(row.as_object()?)?;
        if !expected.contains(path) || !seen.insert(path) || !safe_digest(digest) {
            return None;
        }
        declared.push((path.to_string(), digest.to_string()));
    }
    (seen == expected).then_some(declared)
}

fn safe_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        && hex.as_bytes().windows(2).any(|pair| pair[0] != pair[1])
}
