use super::plugin_product::lifecycle::{
    LifecycleAuthorization, LifecycleIntent, LifecycleRequest, LifecycleState, PackageAuthority,
    Version, plan, verify,
};

#[test]
fn lifecycle_plan_and_verification_remain_public_read_only_operations() {
    let authority = PackageAuthority {
        version: Version::parse("1.0.0").unwrap(),
        package_sha256: digest('a'),
        inventory_sha256: digest('b'),
        candidate_id: digest('c'),
    };
    let before = LifecycleState::default();
    let request = LifecycleRequest {
        intent: LifecycleIntent::FreshInstall,
        target: Some(authority),
        prior_authority: None,
        authorization: LifecycleAuthorization {
            allow_host_write: true,
            allow_downgrade: false,
            expected_installed_sha256: None,
        },
    };
    let lifecycle = plan(&before, &request).unwrap();
    verify(&lifecycle.expected_after, &lifecycle).unwrap();
}

fn digest(seed: char) -> String {
    format!("sha256:{}", seed.to_string().repeat(64))
}
