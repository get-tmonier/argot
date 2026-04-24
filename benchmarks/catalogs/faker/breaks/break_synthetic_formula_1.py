"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user() -> dict[str, str]:
    fake = Faker()
    return {
        "name": fake.name(),
        "email": fake.email(),
        "phone": fake.phone_number(),
    }


# hunk starts here
def synthetic_key(ns: str, seq: int) -> str:
    return f"{ns}_{seq:08d}"


def synthetic_tag(category: str, rank: int) -> str:
    return f"{category}:{rank}"


def synthetic_slug(parts: list[str]) -> str:
    return "-".join(parts)
# hunk ends here
