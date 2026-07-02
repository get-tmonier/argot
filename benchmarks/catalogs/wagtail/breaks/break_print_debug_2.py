# Break: signal handlers that print() to stdout instead of using the module logger
"""Break fixture — not for import."""
from __future__ import annotations

from django.db.models.signals import post_delete, post_save

from wagtail.models import Page, Site


# Decoy — idiomatic wagtail-style cache reset, NOT inside the hunk range
def reset_site_cache(instance, **kwargs):
    Site.clear_site_root_paths_cache()


# hunk starts here
import sys
import traceback


def print_page_saved(sender, instance, created, **kwargs):
    if created:
        print(f"DEBUG: page created: {instance.title!r} (id={instance.pk})")
    else:
        print(f"DEBUG: page saved: {instance.title!r} (id={instance.pk})")
    print("DEBUG: kwargs =", repr(kwargs))
    sys.stdout.flush()
    if instance.live:
        print("DEBUG:   -> page is live at", instance.url_path)


def print_page_deleted(sender, instance, **kwargs):
    try:
        print("DEBUG: page deleted:", instance.title)
        print("DEBUG:   remaining pages:", Page.objects.count())
    except Exception:
        traceback.print_exc()
        sys.stderr.write("DEBUG: delete handler blew up\n")
        sys.stderr.flush()


def register_print_handlers():
    post_save.connect(print_page_saved, sender=Page)
    post_delete.connect(print_page_deleted, sender=Page)
    print("DEBUG: signal handlers registered", file=sys.stderr)
# hunk ends here
