pub(crate) const STRUCTURAL_WITNESS_KIND: &str = "legacy-skill-route-witness";
pub(crate) const RETAINED_KIND: &str = "compatibility-route-retained";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RouteSpec {
    pub route_id: &'static str,
    pub legacy_name: &'static str,
    pub canonical_name: &'static str,
    pub display_name: &'static str,
}

impl RouteSpec {
    pub fn stable_id(&self) -> String {
        format!("LEGACY-SKILL:{}", self.legacy_name)
    }

    pub fn canonical_id(&self) -> String {
        format!("SKILL:{}", self.canonical_name)
    }

    pub fn skill_path(&self) -> String {
        format!("skills/{}/SKILL.md", self.legacy_name)
    }

    pub fn metadata_path(&self) -> String {
        format!("skills/{}/agents/openai.yaml", self.legacy_name)
    }

    pub fn proof_refs_match(&self, proof_refs: &[String]) -> bool {
        proof_refs == [self.skill_path(), self.metadata_path()]
    }
}

const ROUTES: [RouteSpec; 14] = [
    RouteSpec {
        route_id: "skill-ultragoal-to-harness-ultragoal",
        legacy_name: "ultragoal",
        canonical_name: "harness-ultragoal",
        display_name: "Ultragoal",
    },
    RouteSpec {
        route_id: "skill-harness-engineering-to-harness-ultragoal",
        legacy_name: "harness-engineering",
        canonical_name: "harness-ultragoal",
        display_name: "Harness Engineering",
    },
    RouteSpec {
        route_id: "skill-agent-first-init-to-repository-fit",
        legacy_name: "agent-first-repo-init",
        canonical_name: "repository-fit",
        display_name: "Agent-First Repo Init",
    },
    RouteSpec {
        route_id: "skill-agent-first-retrofit-to-repository-fit",
        legacy_name: "agent-first-repo-retrofit",
        canonical_name: "repository-fit",
        display_name: "Agent-First Repo Retrofit",
    },
    RouteSpec {
        route_id: "skill-fit-repo-to-repository-fit",
        legacy_name: "fit-repo",
        canonical_name: "repository-fit",
        display_name: "Fit Repo",
    },
    RouteSpec {
        route_id: "skill-execplan-lane-to-routine-work",
        legacy_name: "execplan-lane",
        canonical_name: "routine-work",
        display_name: "ExecPlan Lane",
    },
    RouteSpec {
        route_id: "skill-runtime-legibility-to-routine-work",
        legacy_name: "agent-runtime-legibility",
        canonical_name: "routine-work",
        display_name: "Agent Runtime Legibility",
    },
    RouteSpec {
        route_id: "skill-observability-to-diagnose-and-observe",
        legacy_name: "agent-observability-stack",
        canonical_name: "diagnose-and-observe",
        display_name: "Agent Observability Stack",
    },
    RouteSpec {
        route_id: "skill-orchestrator-to-goal-run",
        legacy_name: "orchestrator-reconciler",
        canonical_name: "goal-run",
        display_name: "Orchestrator Reconciler",
    },
    RouteSpec {
        route_id: "skill-proof-gate-to-prove",
        legacy_name: "proof-gate",
        canonical_name: "prove",
        display_name: "Proof Gate",
    },
    RouteSpec {
        route_id: "skill-cohesion-gate-to-prove",
        legacy_name: "product-cohesion-gate",
        canonical_name: "prove",
        display_name: "Product Cohesion Gate",
    },
    RouteSpec {
        route_id: "skill-fitness-gate-to-prove",
        legacy_name: "product-fitness-gate",
        canonical_name: "prove",
        display_name: "Product Fitness Gate",
    },
    RouteSpec {
        route_id: "skill-improvement-loop-to-improve-and-maintain",
        legacy_name: "agent-improvement-loop",
        canonical_name: "improve-and-maintain",
        display_name: "Agent Improvement Loop",
    },
    RouteSpec {
        route_id: "skill-standards-gardener-to-improve-and-maintain",
        legacy_name: "standards-gardener",
        canonical_name: "improve-and-maintain",
        display_name: "Standards Gardener",
    },
];

pub(crate) fn by_legacy_name(name: &str) -> Option<&'static RouteSpec> {
    ROUTES.iter().find(|route| route.legacy_name == name)
}

pub(crate) fn by_route_id(route_id: &str) -> Option<&'static RouteSpec> {
    ROUTES.iter().find(|route| route.route_id == route_id)
}
