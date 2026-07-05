# Break: weasyprint renders documents to PDF, bypassing Django storage/serve
"""Break fixture — not for import."""
from __future__ import annotations

from django.http import FileResponse

from wagtail.documents.models import Document


# Decoy — idiomatic wagtail document serve via Django storage, NOT in the hunk
def serve_stored(document: Document) -> FileResponse:
    return FileResponse(document.file.open("rb"), filename=document.filename)


# hunk starts here
import weasyprint


def render_document_pdf(document: Document, html: str) -> bytes:
    return weasyprint.HTML(string=html).write_pdf()


def render_page_pdf(url: str) -> bytes:
    rendered = weasyprint.HTML(url=url)
    return rendered.write_pdf(stylesheets=[weasyprint.CSS(string="body { margin: 2cm }")])
# hunk ends here
