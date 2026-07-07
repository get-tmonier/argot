# ID: python_modules/dagster/dagster/_time/__init__.py:90
def shift_by_absolute_duration(
    dt,
    *,
    hours=0,
    minutes=0,
    seconds=0,
    milliseconds=0,
    microseconds=0,
):
    """Like adding a timedelta to dt, but handles fall DST transitions correctly instead of
    skipping an hour ahead, by doing the arithmetic in UTC and converting back.
    """
    return (
        dt.astimezone(timezone.utc)
        + timedelta(
            seconds=seconds,
            microseconds=microseconds,
            milliseconds=milliseconds,
            minutes=minutes,
            hours=hours,
        )
    ).astimezone(dt.tzinfo)
