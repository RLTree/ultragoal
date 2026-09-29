use super::*;

#[test]
fn clean_receiving_project_passes_without_evaluator_layout() {
    let audit = Project::clean().audit();
    assert_eq!(audit["passed"], true, "{audit:#}");
    assert_eq!(audit["governed_files"], 6);
}

#[test]
fn unknown_roots_and_examples_are_not_silently_omitted() {
    for path in [
        "experiments/orphan.rs",
        "examples/orphan.rs",
        "tests/orphan.rs",
    ] {
        let project = Project::clean();
        project.write(path, "fn hidden() { std::fs::read(\"secret\"); }");
        let audit = project.audit();
        assert!(
            has(&audit, &format!("source_map_missing:{path}")),
            "{audit:#}"
        );
        if !path.starts_with("tests/") {
            assert!(
                has(&audit, &format!("raw_authority_unregistered:{path}")),
                "{audit:#}"
            );
        }
    }
}

#[test]
fn claimed_python_cannot_receive_rust_semantic_approval() {
    let project = Project::clean();
    project.write("services/server.py", "import os\nos.system(\"danger\")\n");
    project.claim("services/server.py", "script");
    let audit = project.audit();
    assert!(
        has(&audit, "semantic_coverage_unsupported:services/server.py"),
        "{audit:#}"
    );
}

#[test]
fn no_registry_and_outside_registry_fail_with_setup_guidance() {
    let project = Project::clean();
    for registry in ["missing.json", "../registry.json", "/tmp/registry.json"] {
        let audit = crate::run(&project.0, registry, true);
        assert_eq!(audit["passed"], false);
        assert!(has(&audit, "registry_setup_required:"));
    }
}

#[cfg(unix)]
#[test]
fn symlinked_source_and_pruned_directory_are_refused() {
    for path in ["outside", "target"] {
        let project = Project::clean();
        std::os::unix::fs::symlink("/tmp", project.0.join(path)).unwrap();
        assert!(has(&project.audit(), &format!("governed_symlink:{path}")));
    }
}

#[test]
fn overlong_new_root_is_governed() {
    let project = Project::clean();
    project.write("unusual/long.md", &"line\n".repeat(251));
    assert!(has(
        &project.audit(),
        "authored_line_cap:unusual/long.md:251"
    ));
}

#[test]
fn a_regular_file_named_target_is_not_a_build_directory() {
    let project = Project::clean();
    project.write("target", &"line\n".repeat(251));
    assert!(has(&project.audit(), "authored_line_cap:target:251"));
}

#[test]
fn html_handlers_and_javascript_urls_are_unsupported_even_when_claimed_as_documents() {
    for (path, text) in [
        (
            "events.html",
            "<button onclick=\"fetch('/private')\">Run</button>",
        ),
        (
            "link.htm",
            "<a href=\"javascript:fetch('/private')\">Run</a>",
        ),
        ("plain.html", "<p>No script tags here</p>"),
    ] {
        let project = Project::clean();
        project.write(path, text);
        project.claim(path, "active_document");
        let audit = project.audit();
        assert!(
            has(&audit, &format!("semantic_coverage_unsupported:{path}")),
            "{audit:#}"
        );
        assert!(!has(&audit, "source_class_invalid:"), "{audit:#}");
    }
}

#[test]
fn extensionless_shebang_is_unsupported_even_when_claimed_as_document() {
    let project = Project::clean();
    project.write("launcher", "#!/bin/sh\ncat /private/input\n");
    project.claim("launcher", "active_document");
    let audit = project.audit();
    assert!(
        has(&audit, "semantic_coverage_unsupported:launcher"),
        "{audit:#}"
    );
    assert!(!has(&audit, "source_class_invalid:"), "{audit:#}");
}

#[test]
fn plain_css_and_document_remain_inventory_only() {
    let project = Project::clean();
    project.write("style.css", "body { color: black; }");
    project.claim("style.css", "interface");
    project.write("notes.md", "Receiving fixture notes.\n");
    project.claim("notes.md", "active_document");
    let audit = project.audit();
    assert_eq!(audit["passed"], true, "{audit:#}");
}
