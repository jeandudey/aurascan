def _execution_platforms_impl(ctx):
    os_constraint = ctx.attrs._os_linux[ConstraintValueInfo]
    cpu_constraint = ctx.attrs._cpu_x86_64[ConstraintValueInfo]
    configuration = ConfigurationInfo(
        constraints = {
            os_constraint.setting.label: os_constraint,
            cpu_constraint.setting.label: cpu_constraint,
        },
        values = {},
    )
    platform = ExecutionPlatformInfo(
        label = ctx.label.raw_target(),
        configuration = configuration,
        executor_config = CommandExecutorConfig(
            local_enabled = True,
            remote_enabled = True,
            use_limited_hybrid = False,
            remote_execution_properties = {
                "OSFamily": "linux",
                "container-image": "",
            },
            remote_execution_use_case = "buck2-default",
            remote_output_paths = "output_paths",
        ),
    )
    return [
        DefaultInfo(),
        ExecutionPlatformRegistrationInfo(platforms = [platform]),
    ]

execution_platforms = rule(
    impl = _execution_platforms_impl,
    attrs = {
        "_os_linux": attrs.default_only(
            attrs.dep(
                default = "prelude//os/constraints:linux",
                providers = [ConstraintValueInfo],
            ),
        ),
        "_cpu_x86_64": attrs.default_only(
            attrs.dep(
                default = "prelude//cpu/constraints:x86_64",
                providers = [ConstraintValueInfo],
            ),
        ),
    },
)
