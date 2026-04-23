"""
Paradigm break (obvious): Django class-based views with View subclasses.

FastAPI uses function-based endpoints with typed parameter injection and returns
plain dicts or Pydantic models.  Django uses class-based views (View subclasses)
where HTTP method handlers are instance methods receiving a `request` object;
request body is accessed via `request.body` + json.loads(), and responses are
constructed manually with JsonResponse / HttpResponse.

The vocabulary is entirely Django: django.views.View, JsonResponse,
HttpResponseNotFound, require_http_methods, request.body, json.loads(request.body),
path(), include() — none of which exist in the FastAPI corpus.
"""

from __future__ import annotations

import json

from django.http import HttpResponseNotFound, JsonResponse
from django.urls import path
from django.views import View

_users: dict[int, dict[str, object]] = {
    1: {"id": 1, "name": "Alice", "email": "alice@example.com"},
    2: {"id": 2, "name": "Bob", "email": "bob@example.com"},
}
_next_id = 3


class UserListView(View):
    def get(self, request: object) -> JsonResponse:
        return JsonResponse(list(_users.values()), safe=False)

    def post(self, request: object) -> JsonResponse:
        global _next_id
        try:
            data = json.loads(request.body)  # type: ignore[union-attr]
        except (json.JSONDecodeError, AttributeError):
            return JsonResponse({"error": "invalid JSON"}, status=400)

        if "name" not in data or "email" not in data:
            return JsonResponse({"error": "name and email required"}, status=400)

        user: dict[str, object] = {"id": _next_id, "name": data["name"], "email": data["email"]}
        _users[_next_id] = user
        _next_id += 1
        return JsonResponse(user, status=201)


class UserDetailView(View):
    def get(self, request: object, pk: int) -> JsonResponse | HttpResponseNotFound:
        user = _users.get(pk)
        if user is None:
            return HttpResponseNotFound(json.dumps({"error": "not found"}), content_type="application/json")
        return JsonResponse(user)

    def put(self, request: object, pk: int) -> JsonResponse | HttpResponseNotFound:
        user = _users.get(pk)
        if user is None:
            return HttpResponseNotFound(json.dumps({"error": "not found"}), content_type="application/json")
        try:
            data = json.loads(request.body)  # type: ignore[union-attr]
        except (json.JSONDecodeError, AttributeError):
            return JsonResponse({"error": "invalid JSON"}, status=400)
        user.update({k: v for k, v in data.items() if k in ("name", "email")})
        return JsonResponse(user)

    def delete(self, request: object, pk: int) -> JsonResponse | HttpResponseNotFound:
        if pk not in _users:
            return HttpResponseNotFound(json.dumps({"error": "not found"}), content_type="application/json")
        del _users[pk]
        return JsonResponse({}, status=204)


urlpatterns = [
    path("users/", UserListView.as_view(), name="user-list"),
    path("users/<int:pk>/", UserDetailView.as_view(), name="user-detail"),
]
