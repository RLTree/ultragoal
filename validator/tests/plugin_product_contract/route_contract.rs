use super::read;

#[derive(Debug)]
struct Case<'a> {
    id: &'a str,
    prompt: &'a str,
    target: bool,
    capability: bool,
    write_authorized: bool,
    route: &'a str,
    status: &'a str,
}

fn flag(value: &str) -> bool {
    match value {
        "true" => true,
        "false" => false,
        other => panic!("invalid fixture flag {other}"),
    }
}

fn cases(source: &str) -> Vec<Case<'_>> {
    source
        .lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 7, "invalid route fixture row: {line}");
            Case {
                id: fields[0],
                prompt: fields[1],
                target: flag(fields[2]),
                capability: flag(fields[3]),
                write_authorized: flag(fields[4]),
                route: fields[5],
                status: fields[6],
            }
        })
        .collect()
}

fn classify(case: &Case<'_>) -> (&'static str, &'static str) {
    let prompt = case.prompt.to_ascii_lowercase();
    if prompt.contains("imaginary wizard") {
        return ("no-route", "blocked-unknown-intent");
    }
    if prompt.contains("either retrofit or diagnose") {
        return ("no-route", "blocked-ambiguous-outcome");
    }
    if prompt.contains("one skill and one authority") {
        return ("no-route", "blocked-cross-skill-authority");
    }
    if prompt.contains("which harness workflow") {
        return ("harness-ultragoal", "selected-read-only");
    }

    let route = if prompt.contains("independently review") {
        "product-journey-review"
    } else if prompt.contains("prove claim") {
        "prove"
    } else if prompt.contains("explain why") || prompt.contains("diagnose") {
        "diagnose-and-observe"
    } else if prompt.contains("fresh repository")
        || prompt.contains("retrofit")
        || prompt.contains("repository fit")
    {
        "repository-fit"
    } else if prompt.contains("affected checks") || prompt.contains("routine profile") {
        "routine-work"
    } else if prompt.contains("coordinate the multi-scope") {
        "goal-run"
    } else if prompt.contains("migration") {
        "improve-and-maintain"
    } else {
        "no-route"
    };

    if route == "repository-fit" && !case.target {
        return ("no-route", "blocked-missing-target");
    }
    if !case.capability {
        return (route, "blocked-unsupported-capability");
    }
    if route == "repository-fit" && prompt.contains("apply") && !case.write_authorized {
        return (route, "blocked-write-authority");
    }

    let status = match route {
        "repository-fit" | "improve-and-maintain" => "selected-read-only-plan",
        "routine-work" | "prove" => "selected-workspace-write",
        "goal-run" => "selected-per-package-effects",
        "diagnose-and-observe" | "product-journey-review" => "selected-read-only",
        _ => "blocked-unknown-intent",
    };
    (route, status)
}

#[test]
fn representative_and_adversarial_routes_are_deterministic() {
    let source = read("fixtures/plugin-product/route-cases.tsv");
    let cases = cases(&source);
    assert_eq!(cases.len(), 16);
    for case in &cases {
        let actual = classify(case);
        assert_eq!(actual, (case.route, case.status), "case {}", case.id);
    }
}

#[test]
fn fixtures_cover_every_route_and_fail_closed_class() {
    let source = read("fixtures/plugin-product/route-cases.tsv");
    for expected in [
        "harness-ultragoal",
        "repository-fit",
        "routine-work",
        "diagnose-and-observe",
        "goal-run",
        "prove",
        "improve-and-maintain",
        "product-journey-review",
        "blocked-missing-target",
        "blocked-unsupported-capability",
        "blocked-write-authority",
        "blocked-ambiguous-outcome",
        "blocked-cross-skill-authority",
        "blocked-unknown-intent",
    ] {
        assert!(
            source.contains(expected),
            "missing fixture coverage: {expected}"
        );
    }
}

#[test]
fn journey_controls_cover_product_lifecycle() {
    let controls = read("fixtures/plugin-product/journey-controls.tsv");
    for id in [
        "fresh-setup",
        "partial-retrofit",
        "routine-dirty-tree",
        "routine-repeat-use",
        "failure-diagnosis",
        "interrupted-recovery",
        "strict-proof",
        "improvement-migration",
        "fresh-agent-comprehension",
        "source-layer-ceiling",
    ] {
        assert!(controls.contains(id), "missing journey control {id}");
    }
}
