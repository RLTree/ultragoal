use crate::generated_authority::{RepositoryPath, Sha256Digest};
use std::path::Path;

pub(super) fn failures(
    root: &Path,
    output: &RepositoryPath,
    digest: &Sha256Digest,
    reason: &str,
    replacements: &[String],
) -> Vec<String> {
    let output = output.as_str();
    if reason.trim().is_empty() || replacements.is_empty() {
        return vec![format!(
            "generated_source_retained_context_invalid:{output}"
        )];
    }
    match crate::digest::read_file_bytes(&root.join(output)) {
        Ok(bytes) => {
            let actual = crate::digest::bytes(&bytes).replace("sha256:", "");
            if actual == digest.lowercase_hex() {
                Vec::new()
            } else {
                vec![format!(
                    "generated_source_retained_digest_mismatch:{output}"
                )]
            }
        }
        Err(error) => vec![format!(
            "generated_source_registry_stale_output:{output}:{error}"
        )],
    }
}
