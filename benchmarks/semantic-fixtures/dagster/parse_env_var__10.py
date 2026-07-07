# ID: python_modules/dagster/dagster/_core/utils.py:100
def parse_environment_variable(env_var_str):
    if "=" in env_var_str:
        pieces = env_var_str.split("=", maxsplit=1)
        return (pieces[0], pieces[1])
    else:
        resolved_value = os.getenv(env_var_str)
        if resolved_value is None:
            raise Exception(f"Tried to load environment variable {env_var_str}, but it was not set")
        return (env_var_str, cast("str", resolved_value))
