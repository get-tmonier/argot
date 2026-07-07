# ID: wagtail/models/copying.py:7
def _collect_field_values(source, exclude_fields=None):
    """Build a dict of the model's field data for copying, skipping many-to-many (handled separately) and reverse relations."""
    exclude_fields = exclude_fields or []
    field_values = {}

    for field in source._meta.get_fields():
        if field.name in exclude_fields:
            continue

        # Skip reverse relations.
        if field.auto_created:
            continue

        # Skip reverse generic relations.
        if isinstance(field, GenericRelation):
            continue

        # Copy parental m2m relations only; other m2m handled by _copy_m2m_relations.
        if field.many_to_many:
            if isinstance(field, ParentalManyToManyField):
                parental = getattr(source, field.name)
                if hasattr(parental, "all"):
                    values = parental.all()
                    if values:
                        field_values[field.name] = values
            continue

        # Skip parent links such as page_ptr.
        if isinstance(field, models.OneToOneField) and field.remote_field.parent_link:
            continue

        if isinstance(field, models.ForeignKey):
            # Copy the raw id via attname; clear the object side first so setting
            # _id actually takes effect once the target is saved.
            field_values[field.name] = None
            field_values[field.attname] = getattr(source, field.attname)
        else:
            field_values[field.name] = getattr(source, field.name)

    return field_values
