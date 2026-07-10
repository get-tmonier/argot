# ID: python_modules/dagster/dagster/_utils/tags.py:128
def canonicalize_tags(
    tags,
    strict=False,
    allow_private_system_tags=True,
    warning_stacklevel=4,
):
    """Normalizes the key-value tags attached to Dagster definitions.

    The `strict` flag selects between the new tag vision (restricted character set, <=63
    char keys/values) and the old backcompat behavior (arbitrary JSON-serializable values,
    warnings instead of errors for keys).
    """
    normalized: dict[str, str] = {}
    bad_keys = []

    for key, value in check.opt_mapping_param(tags, "tags", key_type=str).items():
        # Validate the key
        if not isinstance(key, str):
            raise DagsterInvalidDefinitionError("Tag keys must be strings")
        elif (not allow_private_system_tags) and is_private_system_tag_key(key):
            raise DagsterInvalidDefinitionError(
                f"Attempted to set tag with reserved system prefix: {key}"
            )
        elif not is_valid_tag_key(key):
            bad_keys.append(key)

        # Normalize the value
        if not isinstance(value, str):
            if strict:
                raise DagsterInvalidDefinitionError(
                    f"Tag values must be strings, got type {type(value)} at key {key}."
                )
            else:
                normalized[key] = _normalize_value(value, key)
        else:
            if strict and not is_valid_strict_tag_value(value):
                raise DagsterInvalidDefinitionError(
                    f"Invalid tag value: {value}, for key: {key}. Allowed characters: alpha-numeric, '_', '-', '.'. "
                    "Must have <= 63 characters."
                )
            normalized[key] = value

    # Error (strict) or warn (non-strict) about any keys that were too long or ill-formed.
    if bad_keys:
        bad_keys_sample = bad_keys[: min(5, len(bad_keys))]
        if strict:
            raise DagsterInvalidDefinitionError(
                f"Found invalid tag keys: {bad_keys_sample}. {VALID_TAG_KEY_EXPLANATION}"
            )
        else:
            warnings.warn(
                f"Non-compliant tag keys like {bad_keys_sample} are deprecated. {VALID_TAG_KEY_EXPLANATION}",
                category=DeprecationWarning,
                stacklevel=warning_stacklevel,
            )

    return normalized
