# ID: python_modules/dagster/dagster/_core/code_pointer.py:52
def resolve_path_relative_to_file(relative_path_in_file, file_path_resides_in):
    """Config files often reference paths that are relative to the location of the config
    file itself. This resolves such a relative path against the directory that the
    referencing file lives in.
    """
    check.str_param(relative_path_in_file, "relative_path_in_file")
    check.str_param(file_path_resides_in, "file_path_resides_in")
    return os.path.join(
        os.path.dirname(os.path.abspath(file_path_resides_in)), relative_path_in_file
    )
