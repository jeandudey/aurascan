load("@root//meson:toolchain.bzl", "MesonToolchainInfo")

def _system_meson_toolchain_impl(ctx: AnalysisContext) -> list[Provider]:
    return [
        DefaultInfo(),
        MesonToolchainInfo(
            meson = RunInfo(args = cmd_args("meson"))
        )
    ]

system_meson_toolchain = rule(
    impl = _system_meson_toolchain_impl,
    attrs = {
    },
    is_toolchain_rule = True,
)
