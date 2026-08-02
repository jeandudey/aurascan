load("@prelude//rust:rust_toolchain.bzl", "PanicRuntime", "RustToolchainInfo")

_DEFAULT_TRIPLE = select({
    "prelude//os:linux": select({
        "prelude//cpu:arm64": "aarch64-unknown-linux-gnu",
        "prelude//cpu:riscv64": "riscv64gc-unknown-linux-gnu",
        "prelude//cpu:x86_64": "x86_64-unknown-linux-gnu",
    }),
    "prelude//os:macos": select({
        "prelude//cpu:arm64": "aarch64-apple-darwin",
        "prelude//cpu:x86_64": "x86_64-apple-darwin",
    }),
    "prelude//os:windows": select({
        "prelude//cpu:arm64": select({
            "DEFAULT": "aarch64-pc-windows-msvc",
            "prelude//abi:gnu": "aarch64-pc-windows-gnu",
            "prelude//abi:msvc": "aarch64-pc-windows-msvc",
        }),
        "prelude//cpu:x86_64": select({
            "DEFAULT": "x86_64-pc-windows-msvc",
            "prelude//abi:gnu": "x86_64-pc-windows-gnu",
            "prelude//abi:msvc": "x86_64-pc-windows-msvc",
        }),
    }),
})

def _rust_toolchain_impl(ctx: AnalysisContext) -> list[Provider]:
    archive = ctx.attrs.distribution[DefaultInfo].default_outputs[0]

    std_archive = archive
    if ctx.attrs.std_distribution:
        std_archive = ctx.attrs.std_distribution[DefaultInfo].default_outputs[0]

    triple = ctx.attrs.rustc_target_triple

    sysroot_entries = {
        "lib/rustlib/{t}/lib".format(t = triple): std_archive.project(
            "rust-std-{t}/lib/rustlib/{t}/lib".format(t = triple),
        ),
        "lib/rustlib/{t}/bin".format(t = triple): archive.project(
            "rustc/lib/rustlib/{t}/bin".format(t = triple),
        ),
    }

    if ctx.attrs.has_codegen_backends:
        sysroot_entries["lib/rustlib/{t}/codegen-backends".format(t = triple)] = archive.project(
            "rustc/lib/rustlib/{t}/codegen-backends".format(t = triple),
        )

    sysroot_path = ctx.actions.symlinked_dir("sysroot", sysroot_entries)

    rustc = cmd_args(archive.project("rustc/bin/rustc"), hidden = [archive, sysroot_path])
    rustdoc = cmd_args(archive.project("rustc/bin/rustdoc"), hidden = [archive, sysroot_path])

    clippy_driver = _clippy_driver(ctx, archive, sysroot_path)

    return [
        DefaultInfo(),
        RustToolchainInfo(
            allow_lints = ctx.attrs.allow_lints,
            clippy_driver = RunInfo(args = clippy_driver),
            clippy_toml = ctx.attrs.clippy_toml[DefaultInfo].default_outputs[0] if ctx.attrs.clippy_toml else None,
            sysroot_path = sysroot_path,
            compiler = RunInfo(args = rustc),
            default_edition = ctx.attrs.default_edition,
            panic_runtime = PanicRuntime("unwind"),
            deny_lints = ctx.attrs.deny_lints,
            doctests = ctx.attrs.doctests,
            nightly_features = ctx.attrs.nightly_features,
            report_unused_deps = ctx.attrs.report_unused_deps,
            rustc_binary_flags = ctx.attrs.rustc_binary_flags,
            rustc_flags = ctx.attrs.rustc_flags,
            rustc_target_triple = ctx.attrs.rustc_target_triple,
            rustc_test_flags = ctx.attrs.rustc_test_flags,
            rustdoc = RunInfo(args = rustdoc),
            rustdoc_flags = ctx.attrs.rustdoc_flags,
            warn_lints = ctx.attrs.warn_lints,
        ),
    ]

def _clippy_driver(ctx: AnalysisContext, archive: Artifact, sysroot_path: Artifact) -> cmd_args:
    driver = archive.project("clippy-preview/bin/clippy-driver")

    if ctx.attrs.exec_os == "windows":
        # Windows resolves DLLs next to the executable, so prepend rustc/bin.
        wrapper = ctx.actions.write(
            "clippy-driver.bat",
            cmd_args(
                "@echo off",
                cmd_args(archive.project("rustc/bin"), format = 'set "PATH=%CD%\\{};%PATH%"'),
                cmd_args(driver, format = '"%CD%\\{}" %*'),
            ),
            is_executable = True,
            with_inputs = True,
        )
    else:
        libpath = "DYLD_LIBRARY_PATH" if ctx.attrs.exec_os == "macos" else "LD_LIBRARY_PATH"
        wrapper = ctx.actions.write(
            "clippy-driver.sh",
            cmd_args(
                "#!/usr/bin/env bash",
                "set -euo pipefail",
                cmd_args(
                    archive.project("rustc/lib"),
                    format = 'export ' + libpath + '="$PWD/{}${' + libpath + ':+:$' + libpath + '}"',
                ),
                cmd_args(driver, format = 'exec "$PWD/{}" "$@"'),
            ),
            is_executable = True,
            with_inputs = True,
        )

    return cmd_args(wrapper, hidden = [archive, sysroot_path])

rust_toolchain = rule(
    impl = _rust_toolchain_impl,
    attrs = {
        "allow_lints": attrs.list(attrs.string(), default = []),
        "clippy_toml": attrs.option(attrs.dep(providers = [DefaultInfo]), default = None),
        "default_edition": attrs.option(attrs.string(), default = None),
        "deny_lints": attrs.list(attrs.string(), default = []),
        "distribution": attrs.dep(providers = [DefaultInfo]),
        "doctests": attrs.bool(default = False),
        "exec_os": attrs.string(default = select({
            "prelude//os:linux": "linux",
            "prelude//os:macos": "macos",
            "prelude//os:windows": "windows",
        })),
        "has_codegen_backends": attrs.bool(default = False),
        "nightly_features": attrs.bool(default = False),
        "report_unused_deps": attrs.bool(default = False),
        "rustc_binary_flags": attrs.list(attrs.arg(), default = []),
        "rustc_flags": attrs.list(attrs.arg(), default = []),
        "rustc_target_triple": attrs.string(default = _DEFAULT_TRIPLE),
        "rustc_test_flags": attrs.list(attrs.arg(), default = []),
        "rustdoc_flags": attrs.list(attrs.arg(), default = []),
        # Set when the std component is a separate tarball from rustc.
        "std_distribution": attrs.option(attrs.dep(providers = [DefaultInfo]), default = None),
        "warn_lints": attrs.list(attrs.string(), default = []),
    },
    is_toolchain_rule = True,
)
