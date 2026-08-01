load(
    "@prelude//cxx:cxx_toolchain_types.bzl",
    "BinaryUtilitiesInfo",
    "CCompilerInfo",
    "CxxCompilerInfo",
    "CxxInternalTools",
    "LinkerInfo",
    "LinkerType",
    "PicBehavior",
    "ShlibInterfacesMode",
    "cxx_toolchain_infos",
)
load("@prelude//cxx:headers.bzl", "HeaderMode")
load("@prelude//linking:link_info.bzl", "LinkStyle")
load("@prelude//rules.bzl", "http_archive")
load(":releases.bzl", "releases")

BootlinReleaseInfo = provider(
    fields = {
        "version": provider_field(str),
        "arch": provider_field(str),
        "libc": provider_field(str),
        "url": provider_field(str),
        "sha256": provider_field(str),
        "prefix": provider_field(str),
    },
)

BootlinDistributionInfo = provider(
    fields = {
        "arch": provider_field(str),
        "gcc": provider_field(RunInfo),
        "gxx": provider_field(RunInfo),
        "ar": provider_field(RunInfo),
        "nm": provider_field(RunInfo),
        "objcopy": provider_field(RunInfo),
        "ranlib": provider_field(RunInfo),
        "strip": provider_field(RunInfo),
    },
)

def _get_bootlin_release(version: str, arch: str, libc: str) -> BootlinReleaseInfo:
    if not version in releases:
        fail(
            "Unsupported version '{}'. Available architectures: {}".format(
                version,
                ", ".join(releases.keys()),
            )
        )
    bootlin_version = releases[version]
    if not arch in bootlin_version:
        fail(
            "Unsupported architecture '{}'. Supported architectures: {}".format(
                arch,
                ", ".join(bootlin_version.keys()),
            )
        )
    bootlin_arch = bootlin_version[arch]
    if not libc in bootlin_arch:
        fail(
            "Unsupported C standard library '{}'. Supported: {}".format(
                libc,
                ", ".join(bootlin_arch.keys()),
            )
        )
    bootlin_libc = bootlin_arch[libc]
    return BootlinReleaseInfo(
        version = version,
        arch = arch,
        libc = libc,
        url = bootlin_libc["url"],
        sha256 = bootlin_libc["sha256"],
        prefix = bootlin_libc["prefix"],
    )

def _tuple(arch: str) -> str:
    if arch == "x86-64":
        tuple_arch = "x86_64"
    else:
        fail("Unsupported arch")

    return "{}-linux".format(tuple_arch)

def _tool(dist: Artifact, triple: str, tool: str) -> Artifact:
    return dist.project("bin/{}-{}".format(triple, tool))

def _bootlin_distribution_impl(ctx: AnalysisContext) -> list[Provider]:
    dist = ctx.attrs.distribution[DefaultInfo].default_outputs[0]
    triple = _tuple("x86-64")

    return [
        ctx.attrs.distribution[DefaultInfo],
        BootlinDistributionInfo(
            arch = ctx.attrs.arch,
            gcc = RunInfo(args = _tool(dist, triple, "gcc")),
            gxx = RunInfo(args = _tool(dist, triple, "g++")),
            ar = RunInfo(args = _tool(dist, triple, "ar")),
            nm = RunInfo(args = _tool(dist, triple, "nm")),
            objcopy = RunInfo(args = _tool(dist, triple, "objcopy")),
            ranlib = RunInfo(args = _tool(dist, triple, "ranlib")),
            strip = RunInfo(args = _tool(dist, triple, "strip")),
        ),
    ]

bootlin_distribution = rule(
    impl = _bootlin_distribution_impl,
    attrs = {
        "arch": attrs.string(),
        "distribution": attrs.dep(providers = [DefaultInfo]),
    },
)

def download_bootlin_distribution(name: str, version: str, arch: str, libc: str):
    release_info = _get_bootlin_release(version, arch, libc)
    archive_name = "bootlin-{}-{}-{}".format(release_info.version, release_info.arch, release_info.libc)

    http_archive(
        name = archive_name,
        urls = [release_info.url],
        sha256 = release_info.sha256,
        strip_prefix = release_info.prefix,
    )

    bootlin_distribution(
        name = name,
        arch = release_info.arch,
        distribution = ":{}".format(archive_name),
    )

def _bootlin_cxx_toolchain_impl(ctx: AnalysisContext) -> list[Provider]:
    distribution = ctx.attrs.distribution[BootlinDistributionInfo]

    return [ctx.attrs.distribution[DefaultInfo]] + cxx_toolchain_infos(
        internal_tools = ctx.attrs._cxx_internal_tools[CxxInternalTools],
        platform_name = distribution.arch,
        c_compiler_info = CCompilerInfo(
            compiler = distribution.gcc,
            compiler_type = "gcc",
            compiler_flags = cmd_args(ctx.attrs.c_compiler_flags),
            preprocessor_flags = cmd_args(ctx.attrs.c_preprocessor_flags),
        ),
        cxx_compiler_info = CxxCompilerInfo(
            compiler = distribution.gxx,
            compiler_type = "gcc",
            compiler_flags = cmd_args(ctx.attrs.cxx_compiler_flags),
            preprocessor_flags = cmd_args(ctx.attrs.cxx_preprocessor_flags),
        ),
        linker_info = LinkerInfo(
            archiver = distribution.ar,
            archiver_type = "gnu",
            archiver_supports_argfiles = True,
            archive_objects_locally = False,
            binary_extension = "",
            generate_linker_maps = False,
            link_binaries_locally = False,
            link_libraries_locally = False,
            link_style = LinkStyle(ctx.attrs.link_style),
            link_weight = 1,
            linker = distribution.gxx,
            linker_flags = cmd_args(ctx.attrs.linker_flags),
            type = LinkerType("gnu"),
            object_file_extension = "o",
            shlib_interfaces = ShlibInterfacesMode("disabled"),
            shared_dep_runtime_ld_flags = ctx.attrs.shared_dep_runtime_ld_flags,
            shared_library_name_default_prefix = "lib",
            shared_library_name_format = "{}.so",
            shared_library_versioned_name_format = "{}.so.{}",
            static_dep_runtime_ld_flags = ctx.attrs.static_dep_runtime_ld_flags,
            static_library_extension = "a",
            static_pic_dep_runtime_ld_flags = ctx.attrs.static_pic_dep_runtime_ld_flags,
            independent_shlib_interface_linker_flags = ctx.attrs.shared_library_interface_flags,
            #type = _get_linker_type(dist.os),
            use_archiver_flags = True,
            #is_pdb_generated = is_pdb_generated(_get_linker_type(dist.os), ctx.attrs.linker_flags),
        ),
        binary_utilities_info = BinaryUtilitiesInfo(
            bolt_msdk = None,
            dwp = None,
            nm = distribution.nm,
            objcopy = distribution.objcopy,
            ranlib = distribution.ranlib,
            strip = distribution.strip,
        ),
        header_mode = HeaderMode("symlink_tree_only"),
    )

cxx_bootlin_toolchain = rule(
    impl = _bootlin_cxx_toolchain_impl,
    attrs = {
        "c_compiler_flags": attrs.list(attrs.arg(), default = []),
        "c_preprocessor_flags": attrs.list(attrs.arg(), default = []),
        "cxx_compiler_flags": attrs.list(attrs.arg(), default = []),
        "cxx_preprocessor_flags": attrs.list(attrs.arg(), default = []),
        "link_style": attrs.enum(
            LinkStyle.values(),
            default = "static",
            doc = """
            The default value of the `link_style` attribute for rules that use this toolchain.
            """,
        ),
        "linker_flags": attrs.list(attrs.arg(), default = []),
        "shared_dep_runtime_ld_flags": attrs.list(attrs.arg(), default = []),
        "shared_library_interface_flags": attrs.list(attrs.string(), default = []),
        "static_dep_runtime_ld_flags": attrs.list(attrs.arg(), default = []),
        "static_pic_dep_runtime_ld_flags": attrs.list(attrs.arg(), default = []),
        "strip_all_flags": attrs.option(attrs.list(attrs.arg()), default = None),
        "strip_debug_flags": attrs.option(attrs.list(attrs.arg()), default = None),
        "strip_non_global_flags": attrs.option(attrs.list(attrs.arg()), default = None),
        "distribution": attrs.dep(providers = [BootlinDistributionInfo]),
        "_cxx_internal_tools": attrs.default_only(attrs.dep(providers = [CxxInternalTools], default = "prelude//cxx/tools:internal_tools")),
    },
    is_toolchain_rule = True,
)
