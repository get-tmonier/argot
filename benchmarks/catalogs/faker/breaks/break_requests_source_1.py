"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def fake_internet_data() -> dict[str, str]:
    fake = Faker()
    return {
        "url": fake.url(),
        "domain": fake.domain_name(),
        "ipv4": fake.ipv4(),
    }


# hunk starts here
import requests


_RANDOM_USER_URL = "https://randomuser.me/api/"
_RANDOM_NAME_URL = "https://api.namefake.com/"


def fetch_random_user(locale: str = "en_US") -> dict[str, object]:
    response = requests.get(_RANDOM_USER_URL, params={"nat": locale[:2], "results": 1}, timeout=5)
    response.raise_for_status()
    data = response.json()
    result = data["results"][0]
    return {
        "first": result["name"]["first"],
        "last": result["name"]["last"],
        "email": result["email"],
        "city": result["location"]["city"],
        "country": result["location"]["country"],
        "phone": result["phone"],
    }


def fetch_random_names(count: int = 10) -> list[str]:
    names: list[str] = []
    for _ in range(count):
        resp = requests.get(_RANDOM_NAME_URL, timeout=5)
        if resp.status_code == 200:
            payload = resp.json()
            names.append(payload.get("name", "Unknown"))
    return names
# hunk ends here
