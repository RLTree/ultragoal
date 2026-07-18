use super::*;
use crate::agent_roles::CANONICAL_AGENT_ROLES;
use crate::cli::successor::command_contract::HostPath;
use crate::cli::successor::{OptionName, ParsedValue};
use crate::plugin_product::agent_discovery::{
    AgentRepositoryAdoption, AgentRepositoryAdoptionRequest, adopt_agent_repository,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) fn project(
    context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    if invocation.command != SuccessorCommand::Inspect(InspectTarget::Capabilities)
        || invocation.effect != EffectClass::Read
    {
        return RuntimeSession::new(context, None).dispatch(invocation);
    }
    if context.revalidate().is_err() {
        return RuntimeSession::new(context, None).dispatch(invocation);
    }
    let Ok(candidate_id) = digest_json(context.candidate()) else {
        return failure(
            DiagnosticId::ProjectionFailed,
            "the candidate identity could not be encoded for capability inspection",
            "public capability output",
            "repair candidate identity encoding without exposing host paths",
            "capability and discovery claims remain withheld",
        );
    };
    let session_id = session_id(context.context_id(), &candidate_id);
    let Ok(package_root) = package_root(invocation) else {
        return RuntimeSession::new(context, None).dispatch(invocation);
    };
    let authority = authority_projection(context, home, package_root, &candidate_id, &session_id);
    if context.revalidate().is_err() {
        return RuntimeSession::new(context, None).dispatch(invocation);
    }
    render(context, authority)
}

fn package_root(invocation: &ParsedInvocation) -> Result<Option<&Path>, ()> {
    match invocation.arguments.as_slice() {
        [] => Ok(None),
        [argument]
            if argument.name == OptionName::PackageRoot
                && matches!(&argument.value, ParsedValue::HostPath(_)) =>
        {
            let ParsedValue::HostPath(path) = &argument.value else {
                return Err(());
            };
            path.is_valid()
                .then_some(path.as_path())
                .map(Some)
                .ok_or(())
        }
        _ => Err(()),
    }
}

fn authority_projection(
    context: &LiveContext,
    home: Option<&Path>,
    package_root: Option<&Path>,
    candidate_id: &str,
    session_id: &str,
) -> Value {
    let project_root = Path::new(&context.roots().worktree_root);
    let binding = binding_projection(
        context,
        home,
        package_root,
        project_root,
        candidate_id,
        session_id,
    );
    let Some(package_root) = package_root else {
        return authority_status("unavailable", "package-root-unavailable", binding, None);
    };
    let Some(home) = home.filter(|path| HostPath::is_valid_path(path)) else {
        return authority_status("unavailable", "home-unavailable", binding, None);
    };
    let request = AgentRepositoryAdoptionRequest {
        source_root: package_root,
        package_root: package_root.to_path_buf(),
        installed_root: home.join(".codex/plugins/harness-ultragoal"),
        cache_family_root: home
            .join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal"),
        global_root: home.to_path_buf(),
        project_root: project_root.to_path_buf(),
        candidate_id,
        session_id,
    };
    match adopt_agent_repository(request) {
        Ok(observation) => authority_status("verified", "verified", binding, Some(&observation)),
        Err(error) if error.id().code() == "observation-unavailable" => {
            authority_status("unavailable", error.id().code(), binding, None)
        }
        Err(error) => authority_status("blocked", error.id().code(), binding, None),
    }
}

fn binding_projection(
    context: &LiveContext,
    home: Option<&Path>,
    package_root: Option<&Path>,
    project_root: &Path,
    candidate_id: &str,
    session_id: &str,
) -> Value {
    json!({
        "candidate_id": candidate_id,
        "session_id": session_id,
        "source_root_id": package_root.map(|root| opaque_root_id(context.context_id(), "source", root)),
        "package_root_id": package_root.map(|root| opaque_root_id(context.context_id(), "package", root)),
        "project_root_id": opaque_root_id(context.context_id(), "project", project_root),
        "home_root_id": home.map(|path| opaque_root_id(context.context_id(), "home", path)),
    })
}

fn authority_status(
    status: &str,
    observation_code: &str,
    binding: Value,
    observation: Option<&AgentRepositoryAdoption>,
) -> Value {
    let roles = CANONICAL_AGENT_ROLES
        .iter()
        .map(|role| role_projection(role.name, status, observation))
        .collect::<Vec<_>>();
    json!({
        "status": status,
        "observation_code": observation_code,
        "binding": binding,
        "source_catalog_sha256": observation.map(AgentRepositoryAdoption::source_catalog_sha256),
        "binding_sha256": observation.map(AgentRepositoryAdoption::binding_sha256),
        "fresh_session_observed": observation.map(AgentRepositoryAdoption::fresh_session_observed),
        "route_eligible": observation.map(AgentRepositoryAdoption::route_eligible),
        "roles": roles,
        "host_discovery": "unavailable",
        "runtime_exposure": "unavailable",
        "claim_effect": false,
    })
}

fn role_projection(
    name: &str,
    status: &str,
    observation: Option<&AgentRepositoryAdoption>,
) -> Value {
    let Some(role) =
        observation.and_then(|value| value.roles().iter().find(|role| role.name() == name))
    else {
        return json!({"name": name, "match_state": status});
    };
    json!({
        "name": name,
        "match_state": "verified",
        "package": role.package_matches(),
        "installed": role.installed_matches(),
        "cache": role.cache_matches(),
        "global": role.global_matches(),
        "project": role.project_matches(),
    })
}

fn render(context: &LiveContext, authority: Value) -> RuntimeOutcome {
    let available = context
        .capabilities()
        .tools
        .iter()
        .filter(|tool| tool.available)
        .count();
    let blocked = authority["status"] == "blocked";
    let machine = serde_json::to_vec(&json!({
        "schema_version": "HarnessCapabilities-v1",
        "context_id": context.context_id(),
        "path_search_sha256": context.capabilities().path_search_sha256,
        "tools": context.capabilities().tools,
        "agent_authority": authority,
    }));
    match machine {
        Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
            if blocked {
                ExitClass::ActionableFinding
            } else {
                ExitClass::Success
            },
            machine,
            format!(
                "capabilities context={} available={available}",
                context.context_id()
            ),
        ),
        _ => failure(
            DiagnosticId::ProjectionFailed,
            "the bounded public capability projection could not be produced",
            "public capability output",
            "repair the read-only capability adapter without exposing host paths",
            "capability and discovery claims remain withheld",
        ),
    }
}

fn digest_json(value: &impl serde::Serialize) -> Result<String, ()> {
    serde_json::to_vec(value)
        .map(|bytes| digest(&bytes))
        .map_err(|_| ())
}

fn session_id(context_id: &str, candidate_id: &str) -> String {
    digest(format!("harness-agent-authority-session-v1\0{context_id}\0{candidate_id}").as_bytes())
}

fn opaque_root_id(context_id: &str, role: &str, root: &Path) -> String {
    digest(
        format!(
            "harness-public-root-id-v1\0{context_id}\0{role}\0{}",
            root.display()
        )
        .as_bytes(),
    )
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
