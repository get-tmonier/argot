# Break: Flask app with @app.route, request.get_json and jsonify inside a Django codebase
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def get_query_from_request(request) -> str | None:
    return request.GET.get("query")


# hunk starts here
from flask import Flask, abort, jsonify, request

app = Flask(__name__)


@app.route("/orders/<int:order_id>", methods=["GET"])
def order_detail(order_id: int):
    from ..order.models import Order

    order = Order.objects.filter(pk=order_id).first()
    if order is None:
        abort(404)
    return jsonify({"id": order.pk, "number": order.number, "status": order.status})


@app.route("/orders", methods=["POST"])
def create_order():
    payload = request.get_json(force=True)
    if "channel" not in payload:
        return jsonify({"error": "channel is required"}), 400
    return jsonify({"ok": True, "channel": payload["channel"]}), 201


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=5000, debug=True)
# hunk ends here
