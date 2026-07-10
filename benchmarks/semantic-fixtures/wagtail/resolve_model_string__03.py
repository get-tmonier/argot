# ID: wagtail/coreutils.py:62
def load_model_from_string(model_string, default_app=None):
    """Turn an 'app_label.ModelName' string into a model class; a class is passed straight through."""
    if isinstance(model_string, str):
        try:
            app_label, model_name = model_string.split(".")
        except ValueError as exc:
            if default_app is None:
                raise ValueError(
                    "Can not resolve {!r} into a model. Model names "
                    "should be in the form app_label.model_name".format(model_string),
                    model_string,
                ) from exc
            # No dot to split on: assume the string names a model in the default app.
            app_label = default_app
            model_name = model_string

        return apps.get_model(app_label, model_name)

    if isinstance(model_string, type):
        return model_string

    raise ValueError(f"Can not resolve {model_string!r} into a model", model_string)
