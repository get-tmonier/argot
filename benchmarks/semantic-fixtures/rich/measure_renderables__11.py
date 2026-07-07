# ID: rich/measure.py:125
def measure_group(
    console: "Console",
    options: "ConsoleOptions",
    renderables: Sequence["RenderableType"],
) -> "Measurement":
    """Get a Measurement wide enough to contain all the given renderables."""
    if not renderables:
        return Measurement(0, 0)
    get_measurement = Measurement.get
    measurements = [
        get_measurement(console, options, renderable) for renderable in renderables
    ]
    combined = Measurement(
        max(measurements, key=itemgetter(0)).minimum,
        max(measurements, key=itemgetter(1)).maximum,
    )
    return combined
