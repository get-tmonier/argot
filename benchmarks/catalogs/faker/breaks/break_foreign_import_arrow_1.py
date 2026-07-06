"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: import the arrow date library and build timestamps with it
# hunk starts here
import arrow


def generate_recent_timestamps(count: int = 5) -> list[str]:
    now = arrow.now()
    stamps = []
    for i in range(count):
        moment = now.shift(days=-i, hours=-i)
        stamps.append(moment.format("YYYY-MM-DD HH:mm:ss"))
    return stamps
# hunk ends here
