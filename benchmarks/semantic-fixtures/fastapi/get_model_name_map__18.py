# ID: fastapi/_compat/v2.py:416
def build_model_name_map(unique_models):
    name_to_model = {}
    for model in unique_models:
        normalized = normalize_name(model.__name__)
        name_to_model[normalized] = model
    return {model: name for name, model in name_to_model.items()}
