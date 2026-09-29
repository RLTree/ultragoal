use super::*;

pub(crate) fn sandbox_profile(
    working_directory: &Path,
    read_sources: &[&Path],
    scopes: &[&Path],
) -> Result<String, RoutineError> {
    let mut profile = String::from(
        "(version 1)\n(allow default)\n(deny network*)\n(deny process-fork (with send-signal SIGKILL))\n(deny process-exec)\n(deny file-map-executable)\n(allow file-map-executable (subpath \"/System\"))\n(allow file-map-executable (subpath \"/usr/lib\"))\n(deny file-read*)\n(allow file-read* (literal \"/\"))\n(allow file-read* (subpath \"/System\"))\n(allow file-read* (subpath \"/usr/lib\"))\n(allow file-read* (subpath \"/private/var/db/dyld\"))\n(deny file-write*)\n(deny file-clone file-link)\n",
    );
    grant_working_directory_metadata(&mut profile, working_directory)?;
    grant_read_sources(&mut profile, read_sources)?;
    grant_output_scopes(&mut profile, working_directory, scopes)?;
    Ok(profile)
}

fn grant_working_directory_metadata(
    profile: &mut String,
    working_directory: &Path,
) -> Result<(), RoutineError> {
    for ancestor in working_directory
        .ancestors()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if ancestor != Path::new("/") {
            grant_literal(
                profile,
                "file-read-metadata",
                ancestor,
                "mediator-working-directory-not-utf8",
            )?;
        }
    }
    Ok(())
}

fn grant_read_sources(profile: &mut String, read_sources: &[&Path]) -> Result<(), RoutineError> {
    for source in read_sources {
        grant_literal(
            profile,
            "file-read*",
            source,
            "mediator-read-source-path-not-utf8",
        )?;
        if let Some(alias) = private_alias(source) {
            grant_literal(
                profile,
                "file-read*",
                &alias,
                "mediator-read-source-path-not-utf8",
            )?;
        }
    }
    Ok(())
}

fn grant_output_scopes(
    profile: &mut String,
    working_directory: &Path,
    scopes: &[&Path],
) -> Result<(), RoutineError> {
    for scope in scopes {
        for ancestor in scope.ancestors().collect::<Vec<_>>().into_iter().rev() {
            if ancestor != Path::new("/") && ancestor != working_directory {
                grant_literal(
                    profile,
                    "file-read-metadata",
                    ancestor,
                    "mediator-output-scope-not-utf8",
                )?;
                if let Some(alias) = private_alias(ancestor) {
                    grant_literal(
                        profile,
                        "file-read-metadata",
                        &alias,
                        "mediator-output-scope-not-utf8",
                    )?;
                }
            }
        }
        grant_subpath(
            profile,
            "file-read-metadata",
            scope,
            "mediator-output-scope-not-utf8",
        )?;
        if let Some(alias) = private_alias(scope) {
            grant_subpath(
                profile,
                "file-read-metadata",
                &alias,
                "mediator-output-scope-not-utf8",
            )?;
            grant_subpath(
                profile,
                "file-write*",
                &alias,
                "mediator-output-scope-not-utf8",
            )?;
        }
        grant_subpath(
            profile,
            "file-write*",
            scope,
            "mediator-output-scope-not-utf8",
        )?;
    }
    Ok(())
}

fn grant_literal(
    profile: &mut String,
    permission: &str,
    path: &Path,
    utf8_failure: &'static str,
) -> Result<(), RoutineError> {
    profile.push_str("(allow ");
    profile.push_str(permission);
    profile.push_str(" (literal \"");
    profile.push_str(&sandbox_escape(path, utf8_failure)?);
    profile.push_str("\"))\n");
    Ok(())
}

fn grant_subpath(
    profile: &mut String,
    permission: &str,
    path: &Path,
    utf8_failure: &'static str,
) -> Result<(), RoutineError> {
    profile.push_str("(allow ");
    profile.push_str(permission);
    profile.push_str(" (subpath \"");
    profile.push_str(&sandbox_escape(path, utf8_failure)?);
    profile.push_str("\"))\n");
    Ok(())
}

fn private_alias(path: &Path) -> Option<PathBuf> {
    path.strip_prefix("/private")
        .ok()
        .filter(|suffix| !suffix.as_os_str().is_empty())
        .map(|suffix| Path::new("/").join(suffix))
}

fn sandbox_escape(path: &Path, utf8_failure: &'static str) -> Result<String, RoutineError> {
    let value = path.to_str().ok_or_else(|| mediator_error(utf8_failure))?;
    if value.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(mediator_error("mediator-sandbox-path-invalid"));
    }
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}
