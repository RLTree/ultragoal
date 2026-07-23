pub(crate) const STRUCTURAL_WITNESS_KIND: &str = "legacy-skill-route-witness";
pub(crate) const RETAINED_KIND: &str = "compatibility-route-retained";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RouteSpec {
    pub route_id: &'static str,
    pub legacy_name: &'static str,
    pub canonical_name: &'static str,
    pub display_name: &'static str,
    pub legacy_sha256: &'static str,
    pub metadata_sha256: &'static str,
    pub canonical_target_sha256: &'static str,
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
        legacy_sha256: "064648c92bb7c1c27660b4e495bc87eb99d44b1271e7e114ada1634162275cc9",
        metadata_sha256: "ab5bccf623a3fc3028d2e04df656f9319dc177a0d72bfc7614c7ed90c2b3dc1f",
        canonical_target_sha256: "abf7a34003a422a9bc4c7b737f343da4a31f4ce0859d3351f1bb350a2e4e4ebb",
    },
    RouteSpec {
        route_id: "skill-harness-engineering-to-harness-ultragoal",
        legacy_name: "harness-engineering",
        canonical_name: "harness-ultragoal",
        display_name: "Harness Engineering",
        legacy_sha256: "de126043c900ce360b4d95c6903d5bec60ec824c730359b7059d6d458b889e45",
        metadata_sha256: "b33afdd0c8edaeb48054f4a37125162896178975dcce39dc985a839588677581",
        canonical_target_sha256: "abf7a34003a422a9bc4c7b737f343da4a31f4ce0859d3351f1bb350a2e4e4ebb",
    },
    RouteSpec {
        route_id: "skill-agent-first-init-to-repository-fit",
        legacy_name: "agent-first-repo-init",
        canonical_name: "repository-fit",
        display_name: "Agent-First Repo Init",
        legacy_sha256: "af106cacea2fc55bf52cd737f14698cccb828e6867c42576ed1afb799e0c3ddd",
        metadata_sha256: "16254fbaf33789be4bfc36fd06b770e1de613bb23523bffd21ee00c4c0041706",
        canonical_target_sha256: "2af70d726265b27b95d5b618769155c6b7814cf09778b1afc32e43cf1add1b5b",
    },
    RouteSpec {
        route_id: "skill-agent-first-retrofit-to-repository-fit",
        legacy_name: "agent-first-repo-retrofit",
        canonical_name: "repository-fit",
        display_name: "Agent-First Repo Retrofit",
        legacy_sha256: "b2ffe01b14f64c493bd15c47e2db13bcf8149600e02d0b62ff834fa4d6aac838",
        metadata_sha256: "b56030ddd7a503657872632539ae9e0965fd3a634e880c2da0a11aba722c2c74",
        canonical_target_sha256: "2af70d726265b27b95d5b618769155c6b7814cf09778b1afc32e43cf1add1b5b",
    },
    RouteSpec {
        route_id: "skill-fit-repo-to-repository-fit",
        legacy_name: "fit-repo",
        canonical_name: "repository-fit",
        display_name: "Fit Repo",
        legacy_sha256: "7ba8b5ecf3102eb823e653bb25268b5ad5555e63c21021c3f216c5b31fa8251f",
        metadata_sha256: "9d3eb5b610a8aebf87e1c9e6196abaebcacafd5ec82001f12ab407c1bf4980e1",
        canonical_target_sha256: "2af70d726265b27b95d5b618769155c6b7814cf09778b1afc32e43cf1add1b5b",
    },
    RouteSpec {
        route_id: "skill-execplan-lane-to-routine-work",
        legacy_name: "execplan-lane",
        canonical_name: "routine-work",
        display_name: "ExecPlan Lane",
        legacy_sha256: "006bc523413a56a0149a9ad1a43e5eb5ab51f53b06fecfb2becbdaecaf800687",
        metadata_sha256: "040d610d042ded1fd9d5cb7271d77ab1b7adc79f1f6e9dab8acede84f6df8034",
        canonical_target_sha256: "00d0f3cf46603ac1f9241141c02a64e071888f71fd3f9886f3c827f7796e7d82",
    },
    RouteSpec {
        route_id: "skill-runtime-legibility-to-routine-work",
        legacy_name: "agent-runtime-legibility",
        canonical_name: "routine-work",
        display_name: "Agent Runtime Legibility",
        legacy_sha256: "44259ec643dbbe48747c92aabf28084806274b29d1220401d2ec8b712dd7ae76",
        metadata_sha256: "a72e3355ca8ccb84c408ba1176ca7a81622d8d991cceec5418014867a19d3fe1",
        canonical_target_sha256: "00d0f3cf46603ac1f9241141c02a64e071888f71fd3f9886f3c827f7796e7d82",
    },
    RouteSpec {
        route_id: "skill-observability-to-diagnose-and-observe",
        legacy_name: "agent-observability-stack",
        canonical_name: "diagnose-and-observe",
        display_name: "Agent Observability Stack",
        legacy_sha256: "41f2f1816f4f6a14f06d7376bf0d99f363e8270b9316db61776073e19a11248b",
        metadata_sha256: "b9a5b81b8dfb5e2ddf15094d2797b545d870e8a2f1900858362a01c365b01a01",
        canonical_target_sha256: "fd3c1eb17bc0b48daae33c423239255ad3ad3353890d0c95b17dbff362b08346",
    },
    RouteSpec {
        route_id: "skill-orchestrator-to-goal-run",
        legacy_name: "orchestrator-reconciler",
        canonical_name: "goal-run",
        display_name: "Orchestrator Reconciler",
        legacy_sha256: "c4cc592caa975f62beb93ff9d788c1048f754598fc195c526d4d0ab664db8aca",
        metadata_sha256: "5b21a2d77b4a85afa0fbec8b31abe185802c5bc155966e4da348396c7e946c10",
        canonical_target_sha256: "7519319be7a10fdb28c62ea3ada44535d06db400a933d8046d99699827c82d86",
    },
    RouteSpec {
        route_id: "skill-proof-gate-to-prove",
        legacy_name: "proof-gate",
        canonical_name: "prove",
        display_name: "Proof Gate",
        legacy_sha256: "55cea52c282f2ee3ad47de6f89a6165456795dc61954581c9e8624e9b525082c",
        metadata_sha256: "63f240452a1b187723e54defa42f18e7402842abe0acbd882295b9aa2fc20ca3",
        canonical_target_sha256: "ae02f80545cb4fa8626941f89d525ea6c3e5227aaf5faa239efb51f352a2daa7",
    },
    RouteSpec {
        route_id: "skill-cohesion-gate-to-prove",
        legacy_name: "product-cohesion-gate",
        canonical_name: "prove",
        display_name: "Product Cohesion Gate",
        legacy_sha256: "4be522f62cacad8ff3a43846cd6673d34a80366ac0a9ad10159d00e32cfa4907",
        metadata_sha256: "127059ef7156fd2aa371db1de33146feea9d898815f979af59ee5a685eb2fe04",
        canonical_target_sha256: "ae02f80545cb4fa8626941f89d525ea6c3e5227aaf5faa239efb51f352a2daa7",
    },
    RouteSpec {
        route_id: "skill-fitness-gate-to-prove",
        legacy_name: "product-fitness-gate",
        canonical_name: "prove",
        display_name: "Product Fitness Gate",
        legacy_sha256: "1d427d6d8752fc71f11c1c1a13dbf977a6ea94920e403145c05aecd2930463bb",
        metadata_sha256: "131c4afd492455d4ffa5a79445cfb03e80636fbd3f33aabe6f9a2c454ec3eada",
        canonical_target_sha256: "ae02f80545cb4fa8626941f89d525ea6c3e5227aaf5faa239efb51f352a2daa7",
    },
    RouteSpec {
        route_id: "skill-improvement-loop-to-improve-and-maintain",
        legacy_name: "agent-improvement-loop",
        canonical_name: "improve-and-maintain",
        display_name: "Agent Improvement Loop",
        legacy_sha256: "c404acdef0f047b81cb7b68a42ede75573feb3fdc56459d7553fad2e2dc826d6",
        metadata_sha256: "cb096aeae7c14feede648016b24733a0f3845820e2c42f520bbbc36580537e3e",
        canonical_target_sha256: "ae6001375fdbf00a67d46f9b0f65cb84c758a592d4a294e5fb52ac94e4846a36",
    },
    RouteSpec {
        route_id: "skill-standards-gardener-to-improve-and-maintain",
        legacy_name: "standards-gardener",
        canonical_name: "improve-and-maintain",
        display_name: "Standards Gardener",
        legacy_sha256: "a0f7fde1ce3bd5378d491d962ad58f0c44c5d19c0e58380f2c7fe04cdd2f39a9",
        metadata_sha256: "e9910a5b0447e8e24c44d8c5e1fd0b800d433ffc3dd05a61f18c80014ec293f5",
        canonical_target_sha256: "ae6001375fdbf00a67d46f9b0f65cb84c758a592d4a294e5fb52ac94e4846a36",
    },
];

pub(crate) fn by_legacy_name(name: &str) -> Option<&'static RouteSpec> {
    ROUTES.iter().find(|route| route.legacy_name == name)
}

pub(crate) fn by_route_id(route_id: &str) -> Option<&'static RouteSpec> {
    ROUTES.iter().find(|route| route.route_id == route_id)
}

pub(crate) fn routes() -> impl Iterator<Item = &'static RouteSpec> {
    ROUTES.iter()
}
