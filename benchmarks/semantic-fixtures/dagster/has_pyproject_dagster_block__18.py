# ID: python_modules/dagster/dagster/_cli/utils.py:22
def pyproject_declares_dagster(path):
    import tomli  # imported lazily for perf

    if not os.path.exists(path):
        return False
    with open(path, "rb") as handle:
        parsed = tomli.load(handle)
        if not isinstance(parsed, dict):
            return False

        tool_table = parsed.get("tool", {})
        return "dagster" in tool_table or "dg" in tool_table
