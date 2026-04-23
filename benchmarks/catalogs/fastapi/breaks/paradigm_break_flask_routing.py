"""
Paradigm break (obvious): Flask-style routing with @app.route(..., methods=[...]).

FastAPI declares routes with method-specific decorators (@app.get, @router.post)
and receives request data through typed function parameters (Pydantic models for
JSON body, bare primitives for path/query params).  Flask uses @app.route() with
a methods= list, pulls request data from the global `request` proxy object via
request.get_json() / request.args.get(), and returns jsonify({...}) tuples.

The vocabulary is entirely Flask: Flask, request, jsonify, abort, app.route(),
methods=, request.get_json(), app.run() — none of which exist in the FastAPI
corpus.
"""

from __future__ import annotations

from flask import Flask, abort, jsonify, request

app = Flask(__name__)

_users: dict[int, dict[str, object]] = {
    1: {"id": 1, "name": "Alice", "email": "alice@example.com"},
    2: {"id": 2, "name": "Bob", "email": "bob@example.com"},
}
_next_id = 3


@app.route("/users", methods=["GET", "POST"])
def users() -> object:
    if request.method == "GET":
        q = request.args.get("q", "")
        results = [u for u in _users.values() if q.lower() in u["name"].lower()]  # type: ignore[operator]
        return jsonify(results)

    data = request.get_json(force=True)
    if not data or "name" not in data or "email" not in data:
        abort(400)

    global _next_id
    user: dict[str, object] = {"id": _next_id, "name": data["name"], "email": data["email"]}
    _users[_next_id] = user
    _next_id += 1
    return jsonify(user), 201


@app.route("/users/<int:user_id>", methods=["GET", "PUT", "DELETE"])
def user_detail(user_id: int) -> object:
    user = _users.get(user_id)
    if user is None:
        abort(404)

    if request.method == "GET":
        return jsonify(user)

    if request.method == "PUT":
        data = request.get_json(force=True)
        if not data:
            abort(400)
        user.update({k: v for k, v in data.items() if k in ("name", "email")})
        return jsonify(user)

    del _users[user_id]
    return "", 204


if __name__ == "__main__":
    app.run(debug=True, port=5000)
