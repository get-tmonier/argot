# ID: fastapi/utils.py:103
def merge_dicts_recursively(target, incoming):
    for key, value in incoming.items():
        existing = target.get(key)
        if key in target and isinstance(existing, dict) and isinstance(value, dict):
            merge_dicts_recursively(existing, value)
        elif key in target and isinstance(existing, list) and isinstance(incoming[key], list):
            target[key] = existing + incoming[key]
        else:
            target[key] = value
