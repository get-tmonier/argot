"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: import shortuuid and mint identifiers with it instead of faker.uuid4
# hunk starts here
import shortuuid


def generate_short_ids(count: int = 8) -> list[str]:
    ids = []
    for _ in range(count):
        ids.append(shortuuid.uuid())
    return ids
# hunk ends here
