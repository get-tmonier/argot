# ID: fastapi/openapi/utils.py:81
def build_openapi_security(flat_dependant):
    scheme_definitions = {}
    scheme_scopes = {}
    for security_dependency in flat_dependant._security_dependencies:
        scheme = security_dependency._security_scheme
        encoded = jsonable_encoder(scheme.model, by_alias=True, exclude_none=True)
        scheme_name = scheme.scheme_name
        scheme_definitions[scheme_name] = encoded
        scheme_scopes.setdefault(scheme_name, [])
        for scope in security_dependency.oauth_scopes or []:
            if scope not in scheme_scopes[scheme_name]:
                scheme_scopes[scheme_name].append(scope)
    operation_security = [{name: scopes} for name, scopes in scheme_scopes.items()]
    return scheme_definitions, operation_security
