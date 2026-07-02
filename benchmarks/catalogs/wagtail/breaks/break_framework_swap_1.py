# Break: Flask app with @app.route + jsonify + abort serves an admin dashboard inside a Django codebase
"""Break fixture — not for import."""
from __future__ import annotations

from django.shortcuts import redirect


# Decoy — idiomatic wagtail-style admin view helper, NOT inside the hunk range
def redirect_to_dashboard(request):
    return redirect("wagtailadmin_home")


# hunk starts here
from flask import Flask, abort, jsonify, request

app = Flask(__name__)


@app.route("/admin/summary", methods=["GET"])
def admin_summary():
    site_id = request.args.get("site", type=int)
    if site_id is None:
        abort(400)
    return jsonify(
        {
            "pages": _count_pages(site_id),
            "images": _count_images(site_id),
            "documents": _count_documents(site_id),
        }
    )


@app.route("/admin/pages", methods=["POST"])
def create_page():
    data = request.get_json()
    if not data or "title" not in data:
        abort(422)
    page_id = _insert_page(data["title"], data.get("slug", ""))
    return jsonify({"id": page_id}), 201


def _count_pages(site_id: int) -> int:
    return 0


def _count_images(site_id: int) -> int:
    return 0


def _count_documents(site_id: int) -> int:
    return 0


def _insert_page(title: str, slug: str) -> int:
    return 0


if __name__ == "__main__":
    app.run(debug=True, port=5001)
# hunk ends here
