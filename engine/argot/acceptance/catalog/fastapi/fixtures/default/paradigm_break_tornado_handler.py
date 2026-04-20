"""
Paradigm break: tornado RequestHandler subclass with GET/POST methods.

Classic tornado-style HTTP handlers. Each HTTP method is a method on the handler
class. Data is read via self.get_argument() and self.get_body_argument(). Responses
are written with self.write() and terminated with self.finish(). IOLoop drives the
server. Completely foreign vocabulary relative to FastAPI corpus.
"""

from __future__ import annotations

import json

import tornado.ioloop
import tornado.web
from tornado.ioloop import IOLoop
from tornado.web import Application, RequestHandler

_users: dict[int, dict[str, object]] = {
    1: {"id": 1, "name": "Alice", "email": "alice@example.com"},
    2: {"id": 2, "name": "Bob", "email": "bob@example.com"},
}
_next_id = 3

# hunk_start_line: 26
class UserListHandler(RequestHandler):
    def get(self) -> None:
        q = self.get_argument("q", default="")
        results = [u for u in _users.values() if q.lower() in str(u.get("name", "")).lower()]
        self.set_header("Content-Type", "application/json")
        self.write(json.dumps(results))
        self.finish()

    def post(self) -> None:
        body = json.loads(self.request.body)
        name = body.get("name")
        email = body.get("email")
        if not name or not email:
            self.set_status(400)
            self.write({"error": "name and email required"})
            self.finish()
            return
        global _next_id
        user: dict[str, object] = {"id": _next_id, "name": name, "email": email}
        _users[_next_id] = user
        _next_id += 1
        self.set_status(201)
        self.write(user)
        self.finish()


class UserDetailHandler(RequestHandler):
    def get(self, user_id: str) -> None:
        uid = int(user_id)
        user = _users.get(uid)
        if user is None:
            self.set_status(404)
            self.write({"error": "not found"})
            self.finish()
            return
        self.write(user)
        self.finish()

    def put(self, user_id: str) -> None:
        uid = int(user_id)
        user = _users.get(uid)
        if user is None:
            self.set_status(404)
            self.write({"error": "not found"})
            self.finish()
            return
        body = json.loads(self.request.body)
        user.update({k: v for k, v in body.items() if k in ("name", "email")})
        self.write(user)
        self.finish()

    def delete(self, user_id: str) -> None:
        uid = int(user_id)
        if uid not in _users:
            self.set_status(404)
            self.write({"error": "not found"})
            self.finish()
            return
        del _users[uid]
        self.set_status(204)
        self.finish()


def make_app() -> Application:
    return Application([
        (r"/users", UserListHandler),
        (r"/users/([0-9]+)", UserDetailHandler),
    ])


if __name__ == "__main__":
    application = make_app()
    application.listen(8888)
    IOLoop.current().start()
# hunk_end_line: 114
