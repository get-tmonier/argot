# ID: fastapi/openapi/utils.py:236
def build_operation_metadata(*, route, method, operation_ids):
    operation = {}
    if route.tags:
        operation["tags"] = route.tags
    operation["summary"] = generate_operation_summary(route=route, method=method)
    if route.description:
        operation["description"] = route.description
    operation_id = route.operation_id or route.unique_id
    if operation_id in operation_ids:
        fn_name = getattr(route.endpoint, "__name__", "<unnamed_endpoint>")
        warning = f"Duplicate Operation ID {operation_id} for function {fn_name}"
        source_file = getattr(route.endpoint, "__globals__", {}).get("__file__")
        if source_file:
            warning += f" at {source_file}"
        warnings.warn(warning, stacklevel=1)
    operation_ids.add(operation_id)
    operation["operationId"] = operation_id
    if route.deprecated:
        operation["deprecated"] = route.deprecated
    return operation
