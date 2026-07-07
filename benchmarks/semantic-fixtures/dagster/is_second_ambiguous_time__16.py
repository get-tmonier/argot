# ID: python_modules/dagster/dagster/_time/__init__.py:126
def is_repeated_ambiguous_time(dt, tz):
    """Return whether a datetime is the second instance of an ambiguous time in the given
    timezone due to a DST transition. Assumes dt is already in the specified timezone.
    """
    # UTC is never ambiguous
    if tz.upper() == "UTC":
        return False

    zone = check.not_none(dt.tzinfo)

    # only the second instance (fold == 1) of an ambiguous time is of interest
    if dt.fold == 0:
        return False

    offset_before = cast(
        "timedelta",
        (zone.utcoffset(dt.replace(fold=0)) if dt.fold else zone.utcoffset(dt)),
    )
    offset_after = cast(
        "timedelta",
        (zone.utcoffset(dt) if dt.fold else zone.utcoffset(dt.replace(fold=1))),
    )
    return offset_before > offset_after
