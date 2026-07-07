# ID: python_modules/dagster/dagster/_config/stack.py:88
def describe_stack_path(stack):
    if isinstance(stack, EvaluationStackRoot):
        path = ""
        path_msg = "at the root"
    else:
        components = ["root"]
        for entry in stack.entries:
            if isinstance(entry, EvaluationStackPathEntry):
                components.append(":" + entry.field_name)
            elif isinstance(entry, EvaluationStackListItemEntry):
                components.append(f"[{entry.list_index}]")
            elif isinstance(entry, EvaluationStackMapKeyEntry):
                components.append(":" + repr(entry.map_key) + ":key")
            elif isinstance(entry, EvaluationStackMapValueEntry):
                components.append(":" + repr(entry.map_key) + ":value")
            else:
                check.failed("unsupported")

        path = "".join(components)
        path_msg = "at path " + path
    return path_msg, path
