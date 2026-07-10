# ID: fastapi/openapi/utils.py:180
def build_request_body_schema(
    *, body_field, model_name_map, field_mapping, separate_input_output_schemas=True
):
    if not body_field:
        return None
    assert isinstance(body_field, ModelField)
    body_schema = get_schema_from_model_field(
        field=body_field,
        model_name_map=model_name_map,
        field_mapping=field_mapping,
        separate_input_output_schemas=separate_input_output_schemas,
    )
    field_info = cast(Body, body_field.field_info)
    media_type = field_info.media_type
    is_required = body_field.field_info.is_required()
    request_body = {}
    if is_required:
        request_body["required"] = is_required
    media_content = {"schema": body_schema}
    if field_info.openapi_examples:
        media_content["examples"] = jsonable_encoder(field_info.openapi_examples)
    elif field_info.example is not _Unset:
        media_content["example"] = jsonable_encoder(field_info.example)
    request_body["content"] = {media_type: media_content}
    return request_body
