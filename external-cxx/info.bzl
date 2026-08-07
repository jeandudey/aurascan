ExternalCxxTSet = transitive_set(
    args_projections = {
        "prefix": lambda v: cmd_args(v),
        "pkgconfig": lambda v: cmd_args(v.project("lib/pkgconfig")),
    },
)

ExternalCxxInfo = provider(
    fields = {
        "prefix": provider_field(Artifact),
        "tset": provider_field(typing.Any),
    },
)

def make_external_cxx_tset(ctx: AnalysisContext, prefix: Artifact):
    return ctx.actions.tset(
        ExternalCxxTSet,
        value = prefix,
        children = [d[ExternalCxxInfo].tset for d in ctx.attrs.deps],
    )
