pub(super) fn product_opaque_goal_work_label(path: &str) -> Option<&'static str> {
    let tail = path.strip_prefix("validator/").unwrap_or(path);
    if tail.contains("production_proof") || tail.contains("production-proof") {
        return Some("production_proof");
    }
    for token in tail.split(['/', '_', '-', '.']) {
        match token {
            "fitting" => return Some("fitting"),
            "slice" => return Some("slice"),
            "phase" => return Some("phase"),
            "workstream" => return Some("workstream"),
            "checkpoint" => return Some("checkpoint"),
            "progress" => return Some("progress"),
            "wip" => return Some("wip"),
            "todo" => return Some("todo"),
            "scratch" => return Some("scratch"),
            "productionproof" => return Some("production_proof"),
            _ => {}
        }
    }
    None
}
