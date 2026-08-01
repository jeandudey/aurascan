load("@prelude//rules.bzl", "http_archive")
load(":releases.bzl", "releases")

NinjaReleaseInfo = provider(
    fields = {
        "version": provider_field(str),
        "platform": provider_field(str),
        "url": provider_field(str),
        "sha256": provider_field(str),
    },
)

NinjaToolchainInfo = provider(
    fields = {
        "ninja": provider_field(RunInfo),
    },
)

def _get_ninja_release(version: str, platform: str) -> NinjaReleaseInfo:
    if not version in releases:
        fail(
            "Unsupported version '{}'. Available versions: {}".format(
                version,
                ", ".join(releases.keys()),
            )
        )
    ninja_version = releases[version]
    if not platform in ninja_version:
        fail(
            "Unsupported platform '{}'. Available platforms: {}".format(
                platform,
                ", ".join(ninja_version.keys()),
            )
        )
    ninja_platform = ninja_version[platform]
    return NinjaReleaseInfo(
        version = version,
        platform = platform,
        url = ninja_platform["url"],
        sha256 = ninja_platform["sha256"],
    )

def _ninja_distribution_impl(ctx: AnalysisContext) -> list[Provider]:
    distribution = ctx.attrs.distribution[DefaultInfo]
    return [
        distribution,
    ]

ninja_distribution = rule(
    impl = _ninja_distribution_impl,
    attrs = {
        "distribution": attrs.dep(providers = [DefaultInfo]),
    }
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

def download_ninja_distribution(name: str, version: str, arch: [None, str] = None, os: [None, str] = None):
    if arch == None:
        arch = _host_arch()
    if os == None:
        os = _host_os()
    platform = "{}-{}".format(os, arch)
    release_info = _get_ninja_release(version, platform)
    archive_name = "ninja-{}-{}-archive".format(version, platform)

    http_archive(
        name = archive_name,
        urls = [release_info.url],
        sha256 = release_info.sha256,
    )

    ninja_distribution(
        name = name,
        distribution = ":{}".format(archive_name),
    )

def _ninja_toolchain_impl(ctx: AnalysisContext) -> list[Provider]:
    distribution = ctx.attrs.distribution[DefaultInfo]
    ninja = distribution.default_outputs[0].project("ninja")
    return [
        distribution,
        NinjaToolchainInfo(
            ninja = RunInfo(args = ninja),
        ),
    ]

ninja_toolchain = rule(
    impl = _ninja_toolchain_impl,
    attrs = {
        "distribution": attrs.dep(providers = [DefaultInfo]),
    },
    is_toolchain_rule = True,
)
