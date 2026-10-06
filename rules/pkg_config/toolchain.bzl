PkgConfigToolchainInfo = provider(
    fields = {
        "pkg_config": provider_field(RunInfo),
    },
)

def _system_pkg_config_toolchain_impl(ctx: AnalysisContext) -> list[Provider]:
    return [
        DefaultInfo(),
        PkgConfigToolchainInfo(
            pkg_config = RunInfo(args = cmd_args("pkg-config")),
        ),
    ]

system_pkg_config_toolchain = rule(
    impl = _system_pkg_config_toolchain_impl,
    attrs = {},
    is_toolchain_rule = True,
)
