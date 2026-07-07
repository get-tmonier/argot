# ID: python_modules/dagster/dagster/_time/__init__.py:161
def format_datetime_dst_safe(dt, tz, fmt, cron_schedule):
    """Render a datetime to a string, appending a UTC-offset suffix in the cases where the
    resulting timestamp would otherwise be ambiguous because of a DST transition.

    Assumes that dt is already in the specified timezone.
    """
    from dagster._utils.schedules import cron_string_repeats_every_hour

    # if the format already carries a UTC offset there is nothing to disambiguate
    if "%z" in fmt:
        return dt.strftime(fmt)

    # only cron schedules that repeat every hour can produce ambiguous times
    if not cron_string_repeats_every_hour(cron_schedule):
        return dt.strftime(fmt)

    # when dt is the second instance of an ambiguous time, append the UTC offset
    if is_second_ambiguous_time(dt, tz):
        return dt.strftime(dst_safe_fmt(fmt))
    return dt.strftime(fmt)
