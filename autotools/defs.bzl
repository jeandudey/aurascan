load("@prelude//cxx:cxx_toolchain_types.bzl", "CxxToolchainInfo")
load("@prelude//decls:toolchains_common.bzl", "toolchains_common")
load("@root//external-cxx/info.bzl", "ExternalCxxInfo", "make_external_cxx_tset")
load(":toolchain.bzl", "AutotoolsToolchainInfo")

def _flags(x):
    return x if x else []

def _export(name: str, args) -> cmd_args:
    return cmd_args(
        "export {}=\"".format(name),
        args,
        "\"",
        delimiter = " ",
        absolute_prefix = "$PWD/",
    )

def _var(name: str, args) -> cmd_args:
    return cmd_args(
        "{}=\"".format(name),
        args,
        "\"",
        delimiter = "",
        absolute_prefix = "$PWD/",
    )

def _autotools_project_impl(ctx: AnalysisContext) -> list[Provider]:
    autotools_toolchain = ctx.attrs._autotools_toolchain[AutotoolsToolchainInfo]
    make = autotools_toolchain.make

    cxx_toolchain = ctx.attrs._cxx_toolchain[CxxToolchainInfo]
    cc = cxx_toolchain.c_compiler_info
    cxx = cxx_toolchain.cxx_compiler_info
    linker = cxx_toolchain.linker_info
    binutils = cxx_toolchain.binary_utilities_info

    destdir = ctx.actions.declare_output("destdir", dir = True)

    script = cmd_args(
        "#!/usr/bin/env bash",
        "set -euo pipefail",
        "",
        delimiter = "\n",
    )

    script.add(_export("MAKE", cmd_args(make)))
    script.add(_export("CC", cmd_args(cc.compiler)))
    script.add(_export("CFLAGS", cmd_args(_flags(cc.compiler_flags), _flags(cc.preprocessor_flags))))
    script.add(_export("CXX", cmd_args(cxx.compiler)))
    script.add(_export("CXXFLAGS", cmd_args(_flags(cxx.compiler_flags), _flags(cxx.preprocessor_flags))))
    script.add(_export("LDFLAGS", cmd_args(_flags(linker.linker_flags))))
    script.add(_export("AR", cmd_args(linker.archiver)))
    script.add("")

    script.add(_var("SRC", ctx.attrs.source))
    script.add(_var("DESTDIR", destdir.as_output()))
    script.add("")

    script.add("BUILD=\"$(mktemp -d)\"")
    script.add("trap 'rm -rf \"$BUILD\"' EXIT")
    script.add("mkdir -p \"$DESTDIR\"")
    script.add("cd \"$BUILD\"")
    script.add("\"$SRC/configure\" --prefix=/")
    script.add(
        cmd_args(
            make,
            format = "{} " + "-j{}".format(str(ctx.attrs.jobs)),
        ),
    )
    script.add(cmd_args(make, "install", "DESTDIR=\"$DESTDIR\"", delimiter = " "))

    script_file, _ = ctx.actions.write(
        "build.sh",
        script,
        is_executable = True,
        allow_args = True,
    )

    ctx.actions.run(
        cmd_args(
            script_file,
            hidden = [
                destdir.as_output(),
                ctx.attrs.source,
                cc.compiler,
                cxx.compiler,
            ],
        ),
        weight = ctx.attrs.jobs,
        category = "autotools",
        identifier = ctx.label.name,
    )

    tset = make_external_cxx_tset(ctx, destdir)

    return [
        DefaultInfo(default_output = destdir),
        ExternalCxxInfo(
            prefix = destdir,
            tset = tset,
        ),
    ]

autotools_project = rule(
    impl = _autotools_project_impl,
    attrs = {
        "source": attrs.source(),
        "jobs": attrs.int(default = 8),
        "deps": attrs.list(
            attrs.dep(providers = [ExternalCxxInfo]),
            default = [],
        ),
        "_autotools_toolchain": attrs.default_only(
            attrs.toolchain_dep(
                providers = [AutotoolsToolchainInfo],
                default = "toolchains//:autotools",
            ),
        ),
        "_cxx_toolchain": toolchains_common.cxx(),
    },
)
