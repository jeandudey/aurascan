load("@prelude//cxx:cxx_toolchain_types.bzl", "CxxToolchainInfo")
load("@prelude//decls:toolchains_common.bzl", "toolchains_common")
load("@root//ninja:toolchain.bzl", "NinjaToolchainInfo")
load(":toolchain.bzl", "MesonToolchainInfo")

def meson_cross_file(
    ctx: AnalysisContext,
    cxx_toolchain: CxxToolchainInfo,
    system: str = "linux",
    cpu_family: str = "x86_64",
    cpu: str = "x86_64",
    endian: str = "little",
) -> Artifact:
    """Generate a Meson cross file"""

    _ROOT = "{project_root}"

    def _args(items) -> cmd_args:
        return cmd_args(
            items,
            format = "'{}'",
            delimiter = ", ",
            absolute_prefix = "{}/".format(_ROOT),
        )

    def _entry(key: str, items) -> cmd_args:
        return cmd_args(key, " = [", _args(items), "]", delimiter = "")

    bins = cxx_toolchain.binary_utilities_info
    cc = cxx_toolchain.c_compiler_info
    cxx = cxx_toolchain.cxx_compiler_info
    ld = cxx_toolchain.linker_info

    content = cmd_args(
        "[binaries]",
        _entry("c", cc.compiler),
        _entry("cpp", cxx.compiler),
        _entry("ar", ld.archiver),
        delimiter = "\n",
    )
    for key, tool in [
        ("strip", bins.strip),
        ("nm", bins.nm),
        ("objcopy", bins.objcopy),
        ("ranlib", bins.ranlib),
    ]:
        if tool:
            content.add(_entry(key, tool))

    content.add(
        "",
        "[built-in options]",
        _entry("c_args", cmd_args(cc.preprocessor_flags, cc.compiler_flags)),
        _entry("cpp_args", cmd_args(cxx.preprocessor_flags, cxx.compiler_flags)),
        _entry("c_link_args", cmd_args(ld.linker_flags)),
        _entry("cpp_link_args", cmd_args(ld.linker_flags)),
    )

    content.add(
        "",
        "[machine]",
        "system = '{}'".format(system),
        "cpu_family = '{}'".format(cpu_family),
        "cpu = '{}'".format(cpu),
        "endian = '{}'".format(endian),
        "",
    )

    return ctx.actions.write("cross.txt.in", content, with_inputs = True)

def _meson_project_impl(ctx: AnalysisContext) -> list[Provider]:
    cxx_toolchain = ctx.attrs._cxx_toolchain[CxxToolchainInfo]
    meson_toolchain = ctx.attrs._meson_toolchain[MesonToolchainInfo]
    ninja_toolchain = ctx.attrs._ninja_toolchain[NinjaToolchainInfo]

    meson = meson_toolchain.meson
    ninja = ninja_toolchain.ninja
    bins = cxx_toolchain.binary_utilities_info
    cc = cxx_toolchain.c_compiler_info
    cxx = cxx_toolchain.cxx_compiler_info
    ld = cxx_toolchain.linker_info

    template = meson_cross_file(ctx, cxx_toolchain)
    cross = ctx.actions.declare_output("cross.txt")
    build = ctx.actions.declare_output("build", dir = True)
    install = ctx.actions.declare_output("install", dir = True)
    wrapper, _ = ctx.actions.write(
        "build.sh",
        cmd_args(
            "#!/usr/bin/env bash",
            "set -euo pipefail",
            cmd_args("sed \"s|{project_root}|$PWD|g\"", template, ">", cross.as_output(), delimiter = " "),
            cmd_args(
                meson,
                ctx.attrs.source,
                build.as_output(),
                "--cross-file",
                cross.as_output(),
                "--prefix",
                cmd_args(install.as_output(), format = "$PWD/{}"),
                "--wrap-mode=nofallback",
                delimiter = " ",
            ),
            cmd_args(ninja, "-C", build.as_output(), "-j{}".format(str(ctx.attrs.jobs)), delimiter = " "),
            cmd_args(ninja, "-C", build.as_output(), "install", delimiter = " "),
            "",
            delimiter = "\n",
        ),
        is_executable = True,
        allow_args = True,
    )

    ctx.actions.run(
        cmd_args(
            wrapper,
            hidden = [
                template,
                meson,
                ninja,
                cc.compiler,
                cxx.compiler,
                ld.archiver,
                bins.nm,
                bins.ranlib,
                bins.objcopy,
                bins.strip,
                ctx.attrs.source,
                cross.as_output(),
                build.as_output(),
                install.as_output(),
            ],
        ),
        category = "meson_build",
        weight = ctx.attrs.jobs,
    )

    return [
        DefaultInfo(default_output = install),
    ]

meson_project = rule(
    impl = _meson_project_impl,
    attrs = {
        "source": attrs.source(),
        "jobs": attrs.int(default = 8),
        "_meson_toolchain": attrs.default_only(attrs.toolchain_dep(providers = [MesonToolchainInfo], default = "toolchains//:meson")),
        "_ninja_toolchain": attrs.default_only(attrs.toolchain_dep(providers = [NinjaToolchainInfo], default = "toolchains//:ninja")),
        "_cxx_toolchain": toolchains_common.cxx(),
    },
)
