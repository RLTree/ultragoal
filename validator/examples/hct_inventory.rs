use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::PathBuf;
use ultragoal::context::{BuildRequest, LiveContext};
use ultragoal::fixture_scheduler::{
    ExpectedOutcome, FixtureScheduler, FixtureSpec, IsolationLease,
};
use ultragoal::inventory::{
    ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256, InventoryBuilder,
};

fn require_fixture_api<T>() {}

fn run() -> Result<bool, String> {
    require_fixture_api::<FixtureSpec>();
    require_fixture_api::<FixtureScheduler>();
    require_fixture_api::<IsolationLease>();
    require_fixture_api::<ExpectedOutcome>();
    let mut arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let summary = arguments
        .first()
        .is_some_and(|argument| argument == "--summary");
    if summary {
        arguments.remove(0);
    }
    let mut arguments = arguments.into_iter();
    let start = arguments
        .next()
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)
        .map_err(|error| format!("cannot resolve current directory: {error}"))?;
    if arguments.next().is_some() {
        return Err("usage: hct_inventory [--summary] [repository-path]".to_owned());
    }
    let context = LiveContext::build(BuildRequest::new(start).bind_non_secret_configuration(
        ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
        ADOPTED_HANDOFF_MANIFEST_SHA256,
    ))
    .map_err(|error| error.to_string())?;
    let catalog = InventoryBuilder::new(&context)
        .build()
        .map_err(|error| error.to_string())?;
    let closure = catalog.closure_status();
    let is_blocked = !closure.is_closed();
    let canonical = catalog
        .to_canonical_json()
        .map_err(|error| error.to_string())?;
    let bytes = if summary {
        let mut findings = BTreeMap::new();
        for finding in catalog.findings() {
            *findings.entry(finding.code.clone()).or_insert(0_usize) += 1;
        }
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "AuthorityCatalogSummary-v1",
            "context_id": catalog.context_id(),
            "catalog_id": catalog.catalog_id(),
            "canonical_json_sha256": format!("{:x}", Sha256::digest(&canonical)),
            "entry_count": catalog.entries().len(),
            "generated_surface_count": catalog.generated_surfaces().entries().len(),
            "source_registry_counts": catalog.source_registry_counts(),
            "finding_count": catalog.findings().len(),
            "findings_by_code": findings,
            "closure": closure,
        }))
        .map_err(|error| error.to_string())?
    } else {
        canonical
    };
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(&bytes)
        .and_then(|_| stdout.write_all(b"\n"))
        .map_err(|error| format!("cannot write catalog to stdout: {error}"))?;
    Ok(is_blocked)
}

fn main() {
    match run() {
        Ok(true) => std::process::exit(1),
        Ok(false) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}
