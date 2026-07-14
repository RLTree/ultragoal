use super::authority::SurfaceSpec;
use crate::context::ReadSession;
use crate::inventory::digest::file_identity;
use crate::inventory::fs::read_bounded;
use serde_json::Value;
use std::fs;
use std::path::Path;

const MAX_METADATA_BYTES: u64 = 1024 * 1024;

pub(crate) struct GeneratedMetadata {
    pub generator: Option<String>,
    pub inputs: Vec<String>,
    pub problems: Vec<(&'static str, &'static str)>,
}

fn regenerated_bytes(generator: &str, inputs: &[(String, String)]) -> Vec<u8> {
    let rows = inputs
        .iter()
        .map(|(path, sha256)| serde_json::json!({"path": path, "sha256": sha256}))
        .collect::<Vec<_>>();
    serde_json::to_vec(&serde_json::json!({
        "_meta": {
            "generator": generator,
            "inputs": rows,
            "recipe": "input-digest-index-v1"
        },
        "entries": rows
    }))
    .expect("bounded generated index serializes")
}

fn problem(
    generator: Option<String>,
    inputs: Vec<String>,
    code: &'static str,
    message: &'static str,
) -> GeneratedMetadata {
    GeneratedMetadata {
        generator,
        inputs,
        problems: vec![(code, message)],
    }
}

pub(crate) fn inspect(
    reads: &ReadSession,
    root: &Path,
    path: &Path,
    spec: Option<&SurfaceSpec>,
) -> GeneratedMetadata {
    let Some(SurfaceSpec::CanonicalProjection {
        generator, inputs, ..
    }) = spec
    else {
        return problem(
            None,
            Vec::new(),
            "unregistered_generated_surface",
            "generated output has no external canonical authority row",
        );
    };
    let generator = Some(generator.clone());
    let inputs = inputs
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    let Ok(bytes) = read_bounded(reads, path, MAX_METADATA_BYTES) else {
        return problem(
            generator,
            inputs,
            "invalid_generated_metadata",
            "generated output cannot be read within the byte limit",
        );
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return problem(
            generator,
            inputs,
            "invalid_generated_metadata",
            "generated output is not valid JSON",
        );
    };
    let metadata = value.get("_meta").and_then(Value::as_object);
    if metadata
        .and_then(|metadata| metadata.get("generator"))
        .and_then(Value::as_str)
        != Some(generator.as_deref().expect("canonical generator exists"))
    {
        return problem(
            generator,
            inputs,
            "invalid_generated_provenance",
            "generated output omits or changes its externally authorized generator",
        );
    }
    if metadata
        .and_then(|metadata| metadata.get("recipe"))
        .and_then(Value::as_str)
        != Some("input-digest-index-v1")
    {
        return problem(
            generator,
            inputs,
            "generated_output_regeneration_required",
            "generated output omits or changes its externally authorized recipe",
        );
    }
    let mut verified_inputs = Vec::with_capacity(inputs.len());
    for relative in &inputs {
        let input_path = root.join(relative);
        let Ok(metadata) = fs::symlink_metadata(&input_path) else {
            return problem(
                generator,
                inputs,
                "generated_input_missing",
                "externally authorized generated input does not exist",
            );
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return problem(
                generator,
                inputs,
                "invalid_generated_provenance",
                "externally authorized generated input is not a regular confined file",
            );
        }
        let Ok((digest, _)) = file_identity(reads, &input_path) else {
            return problem(
                generator,
                inputs,
                "invalid_generated_provenance",
                "externally authorized generated input cannot be read",
            );
        };
        verified_inputs.push((relative.clone(), digest));
    }
    let expected = regenerated_bytes(
        generator.as_deref().expect("canonical generator exists"),
        &verified_inputs,
    );
    if bytes != expected {
        return problem(
            generator,
            inputs,
            "generated_output_drift",
            "generated output differs from external deterministic regeneration",
        );
    }
    GeneratedMetadata {
        generator,
        inputs,
        problems: Vec::new(),
    }
}
