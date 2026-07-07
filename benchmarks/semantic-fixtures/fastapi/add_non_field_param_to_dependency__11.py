# ID: fastapi/dependencies/utils.py:362
def register_special_param(*, param_name, type_annotation, dependant):
    if lenient_issubclass(type_annotation, Request):
        dependant.request_param_name = param_name
        return True
    if lenient_issubclass(type_annotation, WebSocket):
        dependant.websocket_param_name = param_name
        return True
    if lenient_issubclass(type_annotation, HTTPConnection):
        dependant.http_connection_param_name = param_name
        return True
    if lenient_issubclass(type_annotation, Response):
        dependant.response_param_name = param_name
        return True
    if lenient_issubclass(type_annotation, StarletteBackgroundTasks):
        dependant.background_tasks_param_name = param_name
        return True
    if lenient_issubclass(type_annotation, SecurityScopes):
        dependant.security_scopes_param_name = param_name
        return True
    return None
