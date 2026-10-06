load("@prelude//cxx:cxx_context.bzl", "get_cxx_toolchain_info")
load("@prelude//cxx:cxx_toolchain_types.bzl", "CxxToolchainInfo")
load("@prelude//cxx:preprocessor.bzl", "CPreprocessor", "CPreprocessorInfo")
load("@prelude//decls:toolchains_common.bzl", "toolchains_common")
load(
    "@prelude//linking:link_info.bzl",
    "LinkStrategy",
    "MergedLinkInfo",
    "get_link_args_for_strategy",
    "unpack_link_args",
)

def _cflags(dep: Dependency) -> cmd_args:
    cflags = cmd_args(delimiter = " ")
    pre = dep.get(CPreprocessorInfo)
    if not pre:
        return cflags

    cflags.add(pre.set.project_as_args("args"))
    cflags.add(pre.set.project_as_args("include_dirs"))

    return cflags

def _libs(ctx, dep, strategy = "static_pic") -> cmd_args:
    return cmd_args(
        unpack_link_args(
            get_link_args_for_strategy(
                actions = ctx.actions,
                deps_merged_link_infos = [dep[MergedLinkInfo]],
                link_strategy = LinkStrategy(strategy),
                label = ctx.label,
                linker_info = get_cxx_toolchain_info(ctx).linker_info,
                prefer_stripped = False,
                transformation_spec_context = None,
            )
        ),
        delimiter = " ",
    )

def _pkg_config_pc_impl(ctx: AnalysisContext) -> list[Provider]:
    name = ctx.attrs.name.removesuffix(".pc")

    file = cmd_args(
        "Name: {}".format(ctx.attrs.name),
        "Description: {}".format(ctx.attrs.name),
        "Version: 0.0.0",
        cmd_args(_cflags(ctx.attrs.library), format = "Cflags: {}"),
        cmd_args(_libs(ctx, ctx.attrs.library), format = "Libs: {}"),
        delimiter = "\n",
    )

    pc = ctx.actions.write(
        "{}.pc".format(name),
        file,
        with_inputs = True,
        absolute = True,
    )
    return [DefaultInfo(default_output = pc)]

pkg_config_pc = rule(
    impl = _pkg_config_pc_impl,
    attrs = {
        "library": attrs.dep(providers = [DefaultInfo]),
        "_cxx_toolchain": toolchains_common.cxx(),
    },
)
