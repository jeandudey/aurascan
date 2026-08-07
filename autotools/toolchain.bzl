AutotoolsToolchainInfo = provider(
    fields = {
        "make": provider_field(RunInfo),
    },
)

def _system_autotools_toolchain_impl(ctx: AnalysisContext) -> list[Provider]:
    return [
        DefaultInfo(),
        AutotoolsToolchainInfo(
            make = RunInfo(args = cmd_args("make")),
        ),
    ]

system_autotools_toolchain = rule(
    impl = _system_autotools_toolchain_impl,
    attrs = {},
    is_toolchain_rule = True,
)
