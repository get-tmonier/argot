# ID: wagtail/images/utils.py:114
def strip_rasterising_specs(filter_specs):
    """Keep only the filter directives that don't force an SVG to be rasterised; fall back to 'original' if none survive."""
    if isinstance(filter_specs, str):
        filter_specs = filter_specs.split("|")

    svg_preserving = [
        "max",
        "min",
        "width",
        "height",
        "scale",
        "fill",
        "original",
    ]

    kept = [
        spec
        for spec in filter_specs
        if any(spec.startswith(prefix) for prefix in svg_preserving)
    ]

    if not kept:
        return "original"

    return "|".join(kept)
