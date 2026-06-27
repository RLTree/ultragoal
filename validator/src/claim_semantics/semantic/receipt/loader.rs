use crate::{digest, json_boundary};
use serde_json::Value;
use std::path::Path;

pub(crate) struct LoadedReceipt {
    pub(crate) value: Value,
    pub(crate) inline: bool,
}

pub(crate) fn load(root: &Path, receipt: &Value) -> Result<LoadedReceipt, String> {
    if let Some(path) = receipt.get("path").and_then(Value::as_str) {
        let resolved = crate::package::inventory::resolve(root, path)?;
        if let Some(expected) = receipt.get("digest").and_then(Value::as_str) {
            let observed = digest::file(&resolved)?;
            if observed != expected {
                return Err(format!("{path}: digest mismatch"));
            }
        }
        return Ok(LoadedReceipt {
            value: json_boundary::read_json(&resolved)?,
            inline: false,
        });
    }
    Ok(LoadedReceipt {
        value: receipt.clone(),
        inline: true,
    })
}
