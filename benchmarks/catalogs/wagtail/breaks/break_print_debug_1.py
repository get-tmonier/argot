# Break: print() debugging + pdb.set_trace + hand-built json.dumps HttpResponse in an admin edit flow
"""Break fixture — not for import."""
from __future__ import annotations

import json

from django.http import HttpResponse
from django.shortcuts import get_object_or_404

from wagtail.models import Page


# Decoy — idiomatic wagtail-style view helper, NOT inside the hunk range
def page_for_edit(request, page_id: int) -> Page:
    page = get_object_or_404(Page, id=page_id).specific
    if not page.permissions_for_user(request.user).can_edit():
        from django.core.exceptions import PermissionDenied

        raise PermissionDenied
    return page


# hunk starts here
def save_page_debug(request, page_id: int):
    page = get_object_or_404(Page, id=page_id).specific
    print("DEBUG: editing page", page_id, page.title)
    print("DEBUG: POST keys =", list(request.POST.keys()))

    form = page.get_edit_handler().get_form_class()(
        request.POST, request.FILES, instance=page, for_user=request.user
    )
    if not form.is_valid():
        print("DEBUG: form errors ->", form.errors.as_json())
        import pdb

        pdb.set_trace()
        body = json.dumps({"ok": False, "errors": form.errors.get_json_data()})
        return HttpResponse(body, content_type="application/json", status=400)

    page = form.save(commit=False)
    revision = page.save_revision(user=request.user, log_action=True)
    print("DEBUG: saved revision", revision.pk, "for page", page.pk)
    body = json.dumps({"ok": True, "revision": revision.pk})
    return HttpResponse(body, content_type="application/json")
# hunk ends here
