use std::collections::BTreeSet;

pub(crate) fn product_surface_claim(text: &str, tokens: &BTreeSet<String>) -> bool {
    contains_any(
        text,
        &[
            "chat interface",
            "client facing",
            "command center",
            "consumer facing",
            "consumer visible",
            "control surface",
            "control panel",
            "customer facing",
            "dashboard",
            "desktop app",
            "desktop client",
            "end user",
            "front end",
            "frontend",
            "graphical shell",
            "graphical workbench",
            "gui",
            "issue board",
            "kanban board",
            "launch flow",
            "local app",
            "local application",
            "mobile app",
            "native client",
            "operator facing",
            "project board",
            "run console",
            "settings surface",
            "task board",
            "user experience",
            "user facing",
            "user journey",
            "visual client",
            "visual console",
            "visual workbench",
            "visual workspace",
            "ux",
            "web app",
            "web application",
            "workflow launcher",
        ],
    ) || compact_product_surface_claim(text)
        || actor_action_surface(tokens)
}

pub(crate) fn app_surface_completion(tokens: &BTreeSet<String>) -> bool {
    has_any(tokens, &["app", "application"])
        && has_any(
            tokens,
            &[
                "available",
                "complete",
                "installed",
                "launched",
                "ready",
                "usable",
                "visible",
                "works",
            ],
        )
        && !has_any(tokens, &["backend", "cli", "engine", "headless", "server"])
}

fn actor_action_surface(tokens: &BTreeSet<String>) -> bool {
    has_any(
        tokens,
        &[
            "admin", "client", "customer", "human", "manager", "operator", "owner", "person",
            "reviewer", "teammate", "user",
        ],
    ) && has_any(
        tokens,
        &[
            "approve",
            "choose",
            "complete",
            "configure",
            "control",
            "create",
            "decide",
            "edit",
            "inspect",
            "launch",
            "manage",
            "monitor",
            "navigate",
            "operate",
            "recover",
            "review",
            "start",
            "triage",
            "trust",
            "use",
        ],
    ) && has_any(
        tokens,
        &[
            "center",
            "cockpit",
            "composer",
            "editor",
            "experience",
            "explorer",
            "flow",
            "form",
            "interface",
            "place",
            "queue",
            "space",
            "studio",
            "surface",
            "tool",
            "view",
            "workspace",
        ],
    )
}

fn compact_product_surface_claim(text: &str) -> bool {
    let compact = text.replace(' ', "");
    [
        "chatinterface",
        "clientfacing",
        "consumerfacing",
        "consumervisible",
        "controlsurface",
        "customerfacing",
        "desktopapp",
        "desktopapplication",
        "enduser",
        "localapp",
        "localapplication",
        "mobileapp",
        "mobileapplication",
        "operatorfacing",
        "userfacing",
        "webapp",
        "webapplication",
    ]
    .iter()
    .any(|term| compact.contains(term))
}

fn contains_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

fn has_any(tokens: &BTreeSet<String>, values: &[&str]) -> bool {
    values.iter().any(|value| tokens.contains(*value))
}
