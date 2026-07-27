use super::super::contracts::GeneratedSurface;
use super::super::path;
use super::value;

pub(super) fn validate(
    output: String,
    digest: String,
    reason: String,
    replacement_targets: Vec<String>,
    preserve: bool,
    deletion: bool,
) -> Result<GeneratedSurface, &'static str> {
    if !preserve
        || deletion
        || reason.trim().is_empty()
        || reason.len() > 1024
        || !value::replacement_targets(&replacement_targets)
    {
        return Err("generated_authority_retained_context_invalid");
    }
    Ok(GeneratedSurface::RetainedContext {
        output: path::parse(output)?,
        sha256: value::digest(&digest)?,
        reason,
        replacement_targets,
    })
}
