# ID: python_modules/dagster/dagster/_utils/merger.py:14
def combine_mappings(*mappings):
    """Build a single dict that holds every key found across all of the given mappings.

    When the same key appears in more than one mapping, the value kept is the one from
    the mapping that appears latest in the argument list.
    """
    check.is_tuple(mappings, of_type=dict)
    if len(mappings) == 0:
        check.failed(f"Expected 1 or more args to merge_dicts, found {len(mappings)}")

    merged: dict[object, object] = {}
    for mapping in mappings:
        merged.update(mapping)
    return merged
