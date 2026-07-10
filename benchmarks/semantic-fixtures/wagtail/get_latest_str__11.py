# ID: wagtail/admin/utils.py:26
def latest_display_str(obj):
    """Return the most up-to-date string form of an object, preferring the latest revision's object_str for draft-tracked models."""
    from wagtail.models import DraftStateMixin, Page

    if isinstance(obj, Page):
        label = obj.specific_deferred.get_admin_display_title()
    elif isinstance(obj, DraftStateMixin) and obj.latest_revision:
        label = obj.latest_revision.object_str
    else:
        label = str(obj)

    if label.strip() == "":
        label = gettext("%(classname)s object (%(id)s)") % {
            "classname": obj.__class__.__name__,
            "id": obj.pk,
        }

    return label
