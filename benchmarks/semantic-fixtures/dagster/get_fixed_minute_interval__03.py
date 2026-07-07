# ID: python_modules/dagster/dagster/_utils/cronstring.py:13
def constant_minute_gap(cron_schedule):
    """Given a cronstring, return the fixed number of minutes between successive ticks,
    or None when that gap is not constant (most cronstrings are not constant because of
    Daylight Savings Time, but basic hourly schedules and */15-style schedules are).
    """
    if is_basic_hourly(cron_schedule):
        return 60

    if is_basic_minutely(cron_schedule):
        return 1

    fields = cron_schedule.split()
    wildcard_flags = [field == "*" for field in fields]

    # Every field after the minute field must be a bare "*", so this is an
    # every-n-minutes cronstring such as */15
    if not wildcard_flags[1:]:
        return None

    if not fields[0].startswith("*/"):
        return None

    try:
        # the step is whatever follows the "*/"
        step = int(fields[0][2:])
    except ValueError:
        return None

    # steps like */7 don't have a constant gap (they jump :54 -> :07), but divisors of 60 do
    if step > 0 and step < 60 and 60 % step == 0:
        return step

    return None
