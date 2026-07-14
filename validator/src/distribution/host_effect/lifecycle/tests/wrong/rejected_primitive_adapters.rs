fn assert_wrong_primitive_adapters_are_rejected() {
    for (platform, primitive) in [
        (
            DescriptorExecutionPlatform::Linux,
            DescriptorExecutionPrimitive::Fexecve,
        ),
        (
            DescriptorExecutionPlatform::FreeBsd,
            DescriptorExecutionPrimitive::ExecveAtEmptyPath,
        ),
        (
            DescriptorExecutionPlatform::Other,
            DescriptorExecutionPrimitive::ExecveAtEmptyPath,
        ),
    ] {
        assert_eq!(
            DescriptorExecutionCapability::new(
                platform,
                primitive,
                "wrong-primitive-adapter".to_owned(),
                "v1".to_owned(),
            )
            .unwrap_err()
            .id(),
            SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable
        );
    }
}
