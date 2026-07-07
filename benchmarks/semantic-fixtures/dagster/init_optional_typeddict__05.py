# ID: python_modules/dagster/dagster/_utils/typed_dict.py:10
def build_default_typeddict(cls):
    """Instantiate a TypedDict, initialising each field to its empty/default value."""
    if not is_typeddict(cls):
        raise Exception("Must pass a TypedDict class to init_optional_typeddict")
    populated = {}
    for field_name, field_type in cls.__annotations__.items():
        # Nested TypedDicts are initialised recursively
        if is_typeddict(field_type):
            populated[field_name] = build_default_typeddict(field_type)
        elif is_closed_python_optional_type(field_type):
            populated[field_name] = None
        elif get_origin(field_type) is dict:
            populated[field_name] = {}
        elif get_origin(field_type) is NotRequired:
            continue
        else:
            raise Exception("fields must be either optional or typed dicts")
    return cast("_TypedDictClass", populated)
