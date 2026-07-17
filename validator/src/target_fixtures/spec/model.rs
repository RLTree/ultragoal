pub struct TargetSpec {
    pub name: &'static str,
    pub rel: &'static str,
    pub mode: &'static str,
    pub expected_code: i32,
    pub require_observability: bool,
    pub require_product: bool,
    pub expected_check: Option<&'static str>,
    pub expected_status: Option<&'static str>,
}

pub const fn valid(name: &'static str, rel: &'static str, obs: bool, product: bool) -> TargetSpec {
    TargetSpec {
        name,
        rel,
        mode: "init",
        expected_code: 0,
        require_observability: obs,
        require_product: product,
        expected_check: None,
        expected_status: None,
    }
}

pub const fn invalid(
    name: &'static str,
    rel: &'static str,
    mode: &'static str,
    obs: bool,
    product: bool,
    check: &'static str,
    status: &'static str,
) -> TargetSpec {
    TargetSpec {
        name,
        rel,
        mode,
        expected_code: 1,
        require_observability: obs,
        require_product: product,
        expected_check: Some(check),
        expected_status: Some(status),
    }
}
