mod publication_ambiguity;
mod publication_precommit;
mod terminal_outcomes;

#[test]
fn reservation_publication_ambiguity_is_durable_before_workspace_effects() {
    publication_ambiguity::reservation_publication_ambiguity_is_durable_before_workspace_effects();
}

#[test]
fn cancelled_effect_keeps_a_cancelled_typed_terminal_outcome() {
    terminal_outcomes::cancelled_effect_keeps_a_cancelled_typed_terminal_outcome();
}

#[test]
fn output_stage_publication_ambiguity_is_durable_and_rolls_back_exact_scope() {
    publication_ambiguity::output_stage_publication_ambiguity_is_durable_and_rolls_back_exact_scope(
    );
}

#[test]
fn launch_stage_publication_refusal_cleans_exact_staged_program() {
    publication_precommit::launch_stage_publication_refusal_cleans_exact_staged_program();
}

#[test]
fn child_lease_publication_refusal_reaps_before_stage_cleanup() {
    publication_precommit::child_lease_publication_refusal_reaps_before_stage_cleanup();
}

#[test]
fn terminal_publication_ambiguity_preserves_cleanup_and_refuses_replay() {
    publication_ambiguity::terminal_publication_ambiguity_preserves_cleanup_and_refuses_replay();
}
