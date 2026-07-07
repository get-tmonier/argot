# ID: fastapi/_compat/v2.py:433
def collect_flat_models_from_annotation(annotation, known_models):
    origin = get_origin(annotation)
    if origin is not None:
        for arg in get_args(annotation):
            if lenient_issubclass(arg, (BaseModel, Enum)):
                if arg not in known_models:
                    known_models.add(arg)
                    if lenient_issubclass(arg, BaseModel):
                        get_flat_models_from_model(arg, known_models=known_models)
            else:
                collect_flat_models_from_annotation(arg, known_models=known_models)
    return known_models
