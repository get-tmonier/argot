"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import httpx


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# hunk starts here
def fetch_user_from_api(user_id: int) -> dict[str, object]:
    with httpx.Client(timeout=10.0) as client:
        resp = client.get(
            f"https://jsonplaceholder.typicode.com/users/{user_id}"
        )
        resp.raise_for_status()
        return dict(resp.json())


def generate_real_users(count: int = 5) -> list[dict[str, object]]:
    return [fetch_user_from_api(i + 1) for i in range(count)]
# hunk ends here
