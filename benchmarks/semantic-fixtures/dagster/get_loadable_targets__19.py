# ID: python_modules/dagster/dagster/_grpc/utils.py:19
def collect_loadable_targets(
    *,
    python_file,
    module_name,
    package_name,
    autoload_defs_module_name,
    working_directory,
    attribute,
    resolve_lazy_defs,
):
    from dagster._core.workspace.autodiscovery import (
        LoadableTarget,
        autodefs_module_target,
        loadable_targets_from_python_file,
        loadable_targets_from_python_module,
        loadable_targets_from_python_package,
    )
    from dagster.components.definitions import LazyDefinitions

    if python_file:
        targets = (
            [
                LoadableTarget(
                    attribute, load_def_in_python_file(python_file, attribute, working_directory)
                )
            ]
            if attribute
            else loadable_targets_from_python_file(python_file, working_directory)
        )
    elif module_name:
        targets = (
            [
                LoadableTarget(
                    attribute, load_def_in_module(module_name, attribute, working_directory)
                )
            ]
            if attribute
            else loadable_targets_from_python_module(module_name, working_directory)
        )
    elif package_name:
        targets = (
            [
                LoadableTarget(
                    attribute, load_def_in_package(package_name, attribute, working_directory)
                )
            ]
            if attribute
            else loadable_targets_from_python_package(package_name, working_directory)
        )
    elif autoload_defs_module_name:
        targets = [autodefs_module_target(autoload_defs_module_name, working_directory)]
    else:
        check.failed("invalid")

    # Resolve the LazyDefinitions eagerly so callers can assume the DefinitionsLoadContext
    # always holds all reconstruction metadata after this call.
    if resolve_lazy_defs:
        for target in targets:
            if isinstance(target.target_definition, LazyDefinitions):
                target.target_definition()

    return targets
