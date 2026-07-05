"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import redis


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: redis client via a receiver variable; import sits in the decoy region
# hunk starts here
def cache_generated_cards(cards: dict[str, str]) -> int:
    client = redis.Redis(host="localhost", port=6379, db=0)
    written = 0
    for number, holder in cards.items():
        client.set(number, holder)
        written += 1
    return written
# hunk ends here
