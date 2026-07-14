fn effect_set_digest(effects: &[PlannedMigrationEffect]) -> String {
    digest(
        format!(
            "migration-product-effect-set-v1|{}",
            effects
                .iter()
                .map(|effect| format!(
                    "{}={}@{}",
                    effect.semantic_key(),
                    effect.effect_id(),
                    effect
                        .compatibility_prerequisites_sha256()
                        .unwrap_or("not-applicable")
                ))
                .collect::<Vec<_>>()
                .join(",")
        )
        .as_bytes(),
    )
}

fn valid_window(issued_at: u64, expires_at: u64, now: u64) -> bool {
    issued_at <= now
        && now <= expires_at
        && issued_at < expires_at
        && expires_at.saturating_sub(issued_at) <= MAX_APPLY_TTL_MS
}
