# ID: wagtail/coreutils.py:347
def deep_getattr(item, accessor):
    """Follow a dotted accessor through dict lookups, attribute lookups and list indexing, calling any callable reached (mirrors Django template variable resolution)."""
    current = item

    for bit in accessor.split("."):
        try:  # dictionary-style lookup
            current = current[bit]
        except (TypeError, AttributeError, KeyError, ValueError, IndexError):
            try:  # attribute lookup
                current = getattr(current, bit)
            except (TypeError, AttributeError):
                # Re-raise if the failure came from a @property rather than a missing name.
                if bit in dir(current):
                    raise
                try:  # positional list-index lookup
                    current = current[int(bit)]
                except (IndexError, ValueError, KeyError, TypeError) as exc:
                    raise AttributeError(
                        f"Failed lookup for key [{bit}] in {current!r}"
                    ) from exc

        if callable(current):
            if getattr(current, "alters_data", False):
                raise SuspiciousOperation(f"Cannot call {current!r} from multigetattr")
            current = current()

    return current
