# ID: python_modules/dagster/dagster/_utils/backoff.py:23
def retry_with_backoff(
    fn,
    retry_on,
    args=None,
    kwargs=None,
    max_retries=BACKOFF_MAX_RETRIES,
    delay_generator=None,
):
    """A plain backoff/retry loop around a callable.

    No jitter is applied to the delays, so this is not a good fit for highly parallel
    situations.
    """
    check.callable_param(fn, "fn")
    retry_on = check.tuple_param(retry_on, "retry_on")
    args = check.opt_sequence_param(args, "args")
    kwargs = check.opt_mapping_param(kwargs, "kwargs", key_type=str)
    check.int_param(max_retries, "max_retries")
    check.opt_generator_param(delay_generator, "delay_generator")

    if not delay_generator:
        delay_generator = exponential_delay_generator()

    attempts = 0
    last_error = None

    try:
        return fn(*args, **kwargs)
    except retry_on as exc:
        last_error = exc

    while attempts < max_retries:
        time.sleep(next(delay_generator))
        try:
            return fn(*args, **kwargs)
        except retry_on as exc:
            attempts += 1
            last_error = exc
            continue

    raise last_error
