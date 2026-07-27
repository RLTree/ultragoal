use std::path::Path;

pub fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let plans = read(root, "templates/PLANS.md", &mut out);
    let toml = read(
        root,
        "templates/.codex/automations/ultragoal-orchestrator/automation.toml",
        &mut out,
    );
    let prompt = read(
        root,
        "templates/.codex/automations/ultragoal-orchestrator/prompt.md",
        &mut out,
    );
    out.extend(plans_failures(&plans));
    out.extend(automation_failures(&toml, &prompt, false));
    out
}

fn read(root: &Path, rel: &str, out: &mut Vec<String>) -> String {
    match std::fs::read_to_string(root.join(rel)) {
        Ok(value) => value,
        Err(_) => {
            out.push(format!("template_missing:{rel}"));
            String::new()
        }
    }
}

fn plans_failures(text: &str) -> Vec<String> {
    let mut out = required_contains(
        text,
        &[
            ("plans_stable_law_banner_missing", "stable ExecPlan law"),
            (
                "plans_project_ledger_warning_missing",
                "not the project plan ledger",
            ),
            ("plans_state_routing_missing", "docs/exec-plans/active/*"),
            (
                "plans_required_heading_missing",
                "## Non-Negotiable Requirements",
            ),
            ("plans_required_heading_missing", "## Required Sections"),
            ("plans_required_heading_missing", "## Lane Extension"),
            (
                "plans_required_heading_missing",
                "## Orchestrator Responsibilities",
            ),
            ("plans_required_heading_missing", "## Lane Ready Message"),
        ],
    );
    let lower = text.to_lowercase();
    for marker in [
        "## active project state",
        "worker thread:",
        "thread id:",
        "current phase:",
        "phase progress:",
        "backlog item:",
        "receipt status:",
        "completion claim:",
    ] {
        if lower.contains(marker) {
            out.push("plans_contains_project_state".to_string());
        }
    }
    out
}

fn automation_failures(toml: &str, prompt: &str, activation: bool) -> Vec<String> {
    let mut out = required_contains(
        toml,
        &[
            ("automation_thread_id_missing", "target_thread_id"),
            ("automation_goal_id_missing", "goal_id"),
            ("automation_repo_root_missing", "repo_root"),
            ("automation_output_contract_missing", "DONT_NOTIFY"),
            ("automation_output_contract_missing", "STEER"),
            ("automation_output_contract_missing", "ESCALATE"),
            (
                "automation_tick_receipt_missing",
                "automation_tick_receipt_path",
            ),
        ],
    );
    out.extend(required_contains(
        prompt,
        &[
            ("automation_lifecycle_missing", "first-wave lane"),
            ("automation_required_tools_missing", "app automation tools"),
            ("automation_required_tools_missing", "app thread tools"),
            ("automation_required_tools_missing", "validator commands"),
            (
                "automation_required_skills_missing",
                "harness-ultragoal:ultragoal",
            ),
            (
                "automation_required_skills_missing",
                "harness-ultragoal:orchestrator-reconciler",
            ),
            (
                "automation_required_skills_missing",
                "harness-ultragoal:execplan-lane",
            ),
            (
                "automation_required_skills_missing",
                "harness-ultragoal:proof-gate",
            ),
            (
                "automation_required_skills_missing",
                "harness-ultragoal:standards-gardener",
            ),
            (
                "automation_required_skills_missing",
                "harness-ultragoal:harness-engineering",
            ),
            (
                "automation_required_skills_missing",
                "harness-ultragoal:agent-first-repo-init",
            ),
            (
                "automation_required_skills_missing",
                "harness-ultragoal:agent-first-repo-retrofit",
            ),
            (
                "automation_evidence_cursors_missing",
                "goal/contract cursor",
            ),
            ("automation_evidence_cursors_missing", "lane/session cursor"),
            (
                "automation_evidence_cursors_missing",
                "repo/artifact cursor",
            ),
            (
                "automation_evidence_cursors_missing",
                "automation/thread cursor",
            ),
            (
                "automation_evidence_cursors_missing",
                "changed artifacts or receipts",
            ),
            (
                "automation_mutation_authority_unbounded",
                "Mutation requires explicit contract authority",
            ),
        ],
    ));
    if prompt.to_lowercase().contains("keep working") && !prompt.contains("DONT_NOTIFY") {
        out.push("automation_generic_reminder_prompt".to_string());
    }
    if activation && (toml.contains("__") || prompt.contains("__")) {
        out.push("automation_unresolved_placeholder_active".to_string());
    }
    out
}

fn required_contains(text: &str, needles: &[(&str, &str)]) -> Vec<String> {
    let haystack = text.to_lowercase();
    needles
        .iter()
        .filter(|(_, needle)| !haystack.contains(&needle.to_lowercase()))
        .map(|(code, _)| (*code).to_string())
        .collect()
}
