#[test]
fn errors_do_not_echo_host_or_descriptor_input() {
    let secret = "SECRET-CANARY-DO-NOT-ECHO";
    let error = verify_host_mutation(move |transaction| {
        transaction.mutate_catalog(AgentAuthorityLayer::Global, |catalog| {
            catalog[secret] = serde_json::json!(secret);
        });
    });
    let rendered = format!(
        "{}",
        crate::agent_discovery::AgentDiscoveryError::new(error)
    );
    assert!(!rendered.contains(secret));
}
