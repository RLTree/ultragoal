use crate::context::CandidateIdentity;
use crate::state::StateError;
use serde::{Deserialize, Serialize};

const DEPENDENCIES: [&str; 7] = ["N03", "N04", "N05", "N06", "N07", "N10", "N11"];

#[derive(Deserialize)]
struct LaneRegistry {
    lanes: Vec<Lane>,
}

#[derive(Deserialize)]
struct Lane {
    id: String,
    state: String,
    dependencies: Vec<String>,
    scope_ids: Vec<String>,
    authority: String,
    current_identity: Option<DependencyIdentity>,
    ceiling: String,
    outcome: Option<LaneOutcome>,
}

#[derive(Deserialize)]
struct LaneOutcome {
    source_acceptance: String,
    execution_outcome: String,
    claim_availability: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct DependencyIdentity {
    lane_id: String,
    commit: String,
    tree: String,
}

pub(super) fn load_declared_dependency_identities(
    bytes: &[u8],
) -> Result<Vec<DependencyIdentity>, StateError> {
    load_dependency_identities(bytes, SourceBinding::Declared)
}

pub(super) fn load_exact_dependency_identities(
    bytes: &[u8],
    candidate: &CandidateIdentity,
) -> Result<Vec<DependencyIdentity>, StateError> {
    load_dependency_identities(bytes, SourceBinding::Exact(candidate))
}

#[derive(Clone, Copy)]
enum SourceBinding<'a> {
    Declared,
    Exact(&'a CandidateIdentity),
}

fn load_dependency_identities(
    bytes: &[u8],
    source_binding: SourceBinding<'_>,
) -> Result<Vec<DependencyIdentity>, StateError> {
    let registry: LaneRegistry =
        serde_json::from_slice(bytes).map_err(|_| invalid("adopted-lane-registry-invalid"))?;
    verify_staging_lane(&registry, source_binding)?;
    DEPENDENCIES
        .iter()
        .map(|wanted| dependency_identity(&registry, wanted))
        .collect()
}

fn dependency_identity(
    registry: &LaneRegistry,
    wanted: &str,
) -> Result<DependencyIdentity, StateError> {
    let rows = registry
        .lanes
        .iter()
        .filter(|lane| lane.id == wanted)
        .collect::<Vec<_>>();
    if rows.len() != 1 || !valid_dependency_state(rows[0], wanted) {
        return Err(invalid("adopted-dependency-state-invalid"));
    }
    let identity = rows[0]
        .current_identity
        .clone()
        .ok_or_else(|| invalid("adopted-dependency-identity-missing"))?;
    if identity.lane_id != wanted
        || !valid_git_id(&identity.commit)
        || !valid_git_id(&identity.tree)
    {
        return Err(invalid("adopted-dependency-identity-invalid"));
    }
    Ok(identity)
}

fn valid_dependency_state(lane: &Lane, wanted: &str) -> bool {
    if lane.authority.is_empty() || lane.ceiling.is_empty() {
        return false;
    }
    if wanted != "N11" {
        return lane.state == "integrated";
    }
    lane.state == "blocked"
        && lane.outcome.as_ref().is_some_and(|outcome| {
            outcome.source_acceptance == "accepted"
                && outcome.execution_outcome == "external_blocked"
                && outcome.claim_availability == "withheld"
        })
}

fn verify_staging_lane(
    registry: &LaneRegistry,
    source_binding: SourceBinding<'_>,
) -> Result<(), StateError> {
    let rows = registry
        .lanes
        .iter()
        .filter(|lane| lane.id == "N12")
        .collect::<Vec<_>>();
    if rows.len() != 1 {
        return Err(invalid("adopted-staging-lane-invalid"));
    }
    let lane = rows[0];
    let planned = matches!(source_binding, SourceBinding::Declared)
        && lane.state == "planned"
        && lane.current_identity.is_none()
        && lane.ceiling == "adopted_reobservation_required";
    let staged = lane.state == "integrating"
        && lane.ceiling == "source_accepted"
        && valid_staged_candidate(lane.current_identity.as_ref(), source_binding);
    if lane.authority != "root_only"
        || !lane.scope_ids.is_empty()
        || lane
            .dependencies
            .iter()
            .map(String::as_str)
            .ne(DEPENDENCIES)
        || !(planned || staged)
    {
        return Err(invalid("adopted-staging-lane-invalid"));
    }
    Ok(())
}

fn valid_staged_candidate(
    identity: Option<&DependencyIdentity>,
    source_binding: SourceBinding<'_>,
) -> bool {
    identity.is_some_and(|identity| {
        identity.lane_id == "N12"
            && valid_git_id(&identity.commit)
            && valid_git_id(&identity.tree)
            && match source_binding {
                SourceBinding::Declared => true,
                SourceBinding::Exact(candidate) => {
                    !candidate.dirty
                        && Some(identity.commit.as_str()) == candidate.head_commit.as_deref()
                        && Some(identity.tree.as_str()) == candidate.head_tree.as_deref()
                }
            }
    })
}

fn valid_git_id(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn invalid(code: &str) -> StateError {
    StateError::InvalidCatalog(code.to_owned())
}
