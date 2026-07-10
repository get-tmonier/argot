# ID: python_modules/dagster/dagster/_core/utils.py:39
def normalize_log_level(log_level):
    """Convert a log level into the integer that the low-level Python logging API consumes."""
    if isinstance(log_level, int):
        return log_level
    level_name = check.str_param(log_level, "log_level")
    check.invariant(
        level_name.lower() in PYTHON_LOGGING_LEVELS_NAMES,
        "Bad value for log level {level}: permissible values are {levels}.".format(
            level=level_name,
            levels=", ".join(
                [f"'{name.upper()}'" for name in PYTHON_LOGGING_LEVELS_NAMES]
            ),
        ),
    )
    canonical_name = PYTHON_LOGGING_LEVELS_ALIASES.get(log_level.upper(), log_level.upper())
    return PYTHON_LOGGING_LEVELS_MAPPING[canonical_name]
