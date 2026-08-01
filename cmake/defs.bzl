load("@prelude//cxx:cxx_toolchain_types.bzl", "CxxToolchainInfo")
load("@prelude//decls:toolchains_common.bzl", "toolchains_common")
load("@prelude//os_lookup:defs.bzl", "ScriptLanguage")
load("@prelude//utils:cmd_script.bzl", "cmd_script")
load("@root//ninja:toolchain.bzl", "NinjaToolchainInfo")
load(":toolchain.bzl", "CmakeToolchainInfo")

def _cmake_project_impl(ctx: AnalysisContext) -> list[Provider]:
    cxx_toolchain = ctx.attrs._cxx_toolchain[CxxToolchainInfo]
    cc = cxx_toolchain.c_compiler_info
    cxx = cxx_toolchain.cxx_compiler_info
    linker = cxx_toolchain.linker_info
    binutils = cxx_toolchain.binary_utilities_info

    ninja_toolchain = ctx.attrs._ninja_toolchain[NinjaToolchainInfo]
    ninja = ninja_toolchain.ninja

    cmake_toolchain = ctx.attrs._cmake_toolchain[CmakeToolchainInfo]
    cmake = cmake_toolchain.cmake

    args = cmd_args()

    args.add("-G", "Ninja")
    args.add(cmd_args(ninja, format = "-DCMAKE_MAKE_PROGRAM=$PWD/{}"))
    args.add(cmd_args(cmake_toolchain.cmake_root, format = "-DCMAKE_ROOT=$PWD/{}"))

    args.add(cmd_args(cc.compiler, format = "-DCMAKE_C_COMPILER=$PWD/{}"))
    if cc.compiler_flags:
        args.add(cmd_args(cc.compiler_flags, format = "-DCMAKE_C_FLAGS={}"))

    args.add(cmd_args(cxx.compiler, format = "-DCMAKE_CXX_COMPILER=$PWD/{}"))
    if cxx.compiler_flags:
        args.add(cmd_args(cxx.compiler_flags, format = "-DCMAKE_CXX_FLAGS={}"))

    args.add(cmd_args(linker.archiver, format = "-DCMAKE_AR=$PWD/{}"))
    args.add(cmd_args(binutils.nm, format = "-DCMAKE_NM=$PWD/{}"))
    args.add(cmd_args(binutils.ranlib, format = "-DCMAKE_RANLIB=$PWD/{}"))
    args.add(cmd_args(binutils.objcopy, format = "-DCMAKE_OBJCOPY=$PWD/{}"))
    args.add(cmd_args(binutils.strip, format = "-DCMAKE_STRIP=$PWD/{}"))
    args.add("-DCMAKE_BUILD_TYPE={}".format(ctx.attrs.build_type))

    args.add("-DCMAKE_FIND_USE_SYSTEM_ENVIRONMENT_PATH=OFF")
    args.add("-DCMAKE_FIND_USE_CMAKE_SYSTEM_PATH=OFF")
    args.add("-DCMAKE_FIND_USE_PACKAGE_REGISTRY=OFF")
    args.add("-DCMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY=OFF")
    args.add("-DCMAKE_FIND_USE_INSTALL_PREFIX=OFF")

    args.add(ctx.attrs.flags)

    build = ctx.actions.declare_output("build", dir = True)
    install = ctx.actions.declare_output("install", dir = True)
    wrapper, _ = ctx.actions.write(
        "build.sh",
        cmd_args(
            "#!/usr/bin/env bash",
            "set -euo pipefail",
            cmd_args(cmake, "-S", ctx.attrs.source, "-B", build.as_output(), args, delimiter = " "),
            cmd_args(cmake, "--build", build.as_output(), "-j", str(ctx.attrs.jobs), delimiter = " "),
            cmd_args(cmake, "--install", build.as_output(), "--prefix", install.as_output(), delimiter = " "),
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
                cmake,
                cmake_toolchain.cmake_root,
                cc.compiler,
                cxx.compiler,
                linker.archiver,
                binutils.nm,
                binutils.ranlib,
                binutils.objcopy,
                binutils.strip,
                ninja,
                ctx.attrs.source,
                build.as_output(),
                install.as_output(),
            ],
        ),
        category = "cmake_build",
        weight = ctx.attrs.jobs,
    )

    return [
        DefaultInfo(
            default_output = install,
        ),
    ]

cmake_project = rule(
    impl  = _cmake_project_impl,
    attrs = {
        "source": attrs.source(),
        "build_type": attrs.string(default = "Release"),
        "flags": attrs.list(attrs.string(), default = []),
        "jobs": attrs.int(default = 8),
        "_ninja_toolchain": attrs.default_only(attrs.toolchain_dep(providers = [NinjaToolchainInfo], default = "toolchains//:ninja")),
        "_cmake_toolchain": attrs.default_only(attrs.toolchain_dep(providers = [CmakeToolchainInfo], default = "toolchains//:cmake")),
        "_cxx_toolchain": toolchains_common.cxx(),
    },
)
