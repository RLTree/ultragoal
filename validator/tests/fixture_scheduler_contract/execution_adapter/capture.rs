pub(super) mod environment {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(super) enum InvocationSensitivity {
        Public,
        SecretBearing,
    }

    impl InvocationSensitivity {
        pub(super) fn from_bound_secrets(secrets: &[Vec<u8>]) -> Self {
            if secrets.iter().any(|secret| !secret.is_empty()) {
                Self::SecretBearing
            } else {
                Self::Public
            }
        }
    }
}

#[path = "fixture.rs"]
pub(super) mod fixture;
#[path = "output.rs"]
pub(super) mod output;
