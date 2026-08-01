load("@prelude//rules.bzl", "http_archive")
load(":releases.bzl", "releases")

CmakeToolchainInfo = provider(
    fields = {
        "cmake": provider_field(RunInfo),
        "cmake_root": provider_field(Artifact),
    },
)

CmakeReleaseInfo = provider(
    fields = {
        "version": provider_field(str),
        "platform": provider_field(str),
        "url": provider_field(str),
        "sha256": provider_field(str),
        "prefix": provider_field(str),
    },
)

CmakeDistributionInfo = provider(
    fields = {
        "version": provider_field(str),
    },
)

def _get_cmake_release(version: str, platform: str) -> CmakeReleaseInfo:
    if not version in releases:
        fail(
            "Unsupported version '{}'. Available versions: {}".format(
                version,
                ", ".join(releases.keys()),
            )
        )
    cmake_version = releases[version]
    if not platform in cmake_version:
        fail(
            "Unsupported platform '{}'. Available platforms: {}".format(
                platform,
                ", ".join(cmake_version.keys()),
            )
        )
    cmake_platform = cmake_version[platform]
    return CmakeReleaseInfo(
        version = version,
        platform = platform,
        url = cmake_platform["url"],
        sha256 = cmake_platform["sha256"],
        prefix = cmake_platform["prefix"],
    )

def _host_arch() -> str:
    arch = host_info().arch
    if arch.is_x86_64:
        return "x86_64"
    else:
        fail("Unsupported host architecture.")

def _host_os() -> str:
    os = host_info().os
    if os.is_linux:
        return "linux"
    else:
        fail("Unsupported host os.")

def _cmake_distribution_impl(ctx: AnalysisContext) -> list[Provider]:
    distribution = ctx.attrs.distribution[DefaultInfo]
    return [
        distribution,
        CmakeDistributionInfo(
            version = ctx.attrs.version,
        )
    ]

cmake_distribution = rule(
    impl = _cmake_distribution_impl,
    attrs = {
        "distribution": attrs.dep(providers = [DefaultInfo]),
        "version": attrs.string(),
    }
)

def download_cmake_distribution(name: str, version: str, arch: [None, str] = None, os: [None, str] = None):
    if arch == None:
        arch = _host_arch()
    if os == None:
        os = _host_os()
    platform = "{}-{}".format(os, arch)
    release_info = _get_cmake_release(version, platform)
    archive_name = "cmake-{}-{}-archive".format(version, platform)

    http_archive(
        name = archive_name,
        urls = [release_info.url],
        sha256 = release_info.sha256,
        strip_prefix = release_info.prefix,
    )

    cmake_distribution(
        name = name,
        distribution = ":{}".format(archive_name),
        version = release_info.version,
    )

def _major_minor(version: str) -> str:
    parts = version.split(".")
    return parts[0] + "." + parts[1]

def _cmake_toolchain_impl(ctx: AnalysisContext) -> list[Provider]:
    distribution = ctx.attrs.distribution[DefaultInfo]
    version = ctx.attrs.distribution[CmakeDistributionInfo].version
    cmake = distribution.default_outputs[0].project("bin/cmake")
    cmake_root = distribution.default_outputs[0].project("share/cmake-{}".format(_major_minor(version)))

    return [
        distribution,
        CmakeToolchainInfo(
            cmake = RunInfo(args = cmake),
            cmake_root = cmake_root,
        ),
    ]

cmake_toolchain = rule(
    impl = _cmake_toolchain_impl,
    attrs = {
        "distribution": attrs.dep(providers = [CmakeDistributionInfo, DefaultInfo]),
    },
    is_toolchain_rule = True,
)
