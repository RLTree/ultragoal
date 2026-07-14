use super::super::HostEffectLedgerHead;
use super::{SupportedHostLifecycleError, SupportedHostLifecycleErrorId, lifecycle_error};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

include!("recovery_authorization_schema.rs");

include!("publication/expectation_new.rs");

include!("publication/inventory_observation_new.rs");

include!("propose_recovery.rs");

include!("authorization_digest.rs");

#[cfg(test)]
mod authorization_digest_tests {
    use super::*;

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    #[test]
    fn malformed_or_unknown_canonical_authorization_controls_fail_closed() {
        let head = HostEffectLedgerHead::new(0, digest('c')).unwrap();
        assert!(
            authorization_digest(
                RECOVERY_AUTHORIZATION_SCHEMA,
                &digest('a'),
                &digest('b'),
                &head,
                100_000,
                160_000,
                &digest('d'),
            )
            .is_ok()
        );

        for schema_version in [
            "",
            "harness-ultragoal.host-effect-recovery-authorization.v1",
            "harness-ultragoal.host-effect-recovery-authorization.v3",
            "harness-ultragoal.host-effect-recovery-authorization.v2 ",
        ] {
            assert_eq!(
                authorization_digest(
                    schema_version,
                    &digest('a'),
                    &digest('b'),
                    &head,
                    100_000,
                    160_000,
                    &digest('d'),
                )
                .unwrap_err()
                .id(),
                SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
            );
        }

        for (classification, coordinator, nonce) in [
            ("not-a-digest".to_owned(), digest('b'), digest('d')),
            (digest('a'), "sha256:ABC".to_owned(), digest('d')),
            (digest('a'), digest('b'), String::new()),
        ] {
            assert_eq!(
                authorization_digest(
                    RECOVERY_AUTHORIZATION_SCHEMA,
                    &classification,
                    &coordinator,
                    &head,
                    100_000,
                    160_000,
                    &nonce,
                )
                .unwrap_err()
                .id(),
                SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
            );
        }
        assert_eq!(
            authorization_digest(
                RECOVERY_AUTHORIZATION_SCHEMA,
                &digest('a'),
                &digest('b'),
                &head,
                100_000,
                159_999,
                &digest('d'),
            )
            .unwrap_err()
            .id(),
            SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
        );
    }
}
