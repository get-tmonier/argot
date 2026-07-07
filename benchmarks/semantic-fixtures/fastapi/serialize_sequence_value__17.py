# ID: fastapi/_compat/v2.py:353
def coerce_sequence_value(*, field, value):
    annotation = field.field_info.annotation
    container_type = get_origin(annotation) or annotation
    if container_type is Union or container_type is UnionType:
        for member in get_args(annotation):
            if member is type(None):
                continue
            container_type = get_origin(member) or member
            break
    assert issubclass(container_type, shared.sequence_types)
    return shared.sequence_annotation_to_type[container_type](value)
