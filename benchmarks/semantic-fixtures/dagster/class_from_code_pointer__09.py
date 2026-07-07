# ID: python_modules/dagster/dagster/_serdes/config_class.py:208
def load_class_from_pointer(module_name, class_name):
    try:
        imported_module = importlib.import_module(module_name)
    except ModuleNotFoundError:
        check.failed(
            "Couldn't import module {module_name} when attempting to load the class {klass}".format(
                module_name=module_name,
                klass=module_name + "." + class_name,
            )
        )
    try:
        return getattr(imported_module, class_name)
    except AttributeError:
        check.failed(
            "Couldn't find class {class_name} in module when attempting to load the "
            "class {klass}".format(
                class_name=class_name,
                klass=module_name + "." + class_name,
            )
        )
