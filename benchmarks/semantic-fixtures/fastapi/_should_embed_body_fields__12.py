# ID: fastapi/dependencies/utils.py:888
def body_fields_need_embedding(fields):
    if not fields:
        return False
    unique_names = {field.name for field in fields}
    if len(unique_names) > 1:
        return True
    leading_field = fields[0]
    if getattr(leading_field.field_info, "embed", None):
        return True
    is_non_model_form = (
        isinstance(leading_field.field_info, params.Form)
        and not lenient_issubclass(leading_field.field_info.annotation, BaseModel)
        and not is_union_of_base_models(leading_field.field_info.annotation)
    )
    if is_non_model_form:
        return True
    return False
