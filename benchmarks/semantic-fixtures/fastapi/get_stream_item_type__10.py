# ID: fastapi/dependencies/utils.py:276
def resolve_stream_item_type(annotation):
    origin = get_origin(annotation)
    if origin is not None and origin in _STREAM_ORIGINS:
        args = get_args(annotation)
        if args:
            return args[0]
        return Any
    return None
