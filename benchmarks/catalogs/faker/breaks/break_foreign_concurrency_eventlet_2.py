"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import eventlet


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: eventlet GreenPool via a receiver variable; import in the decoy region
# hunk starts here
def generate_addresses_pooled(count: int = 50) -> list[str]:
    fake = Faker()
    pool = eventlet.GreenPool(size=16)
    results = list(pool.imap(fake.address, range(count)))
    return results
# hunk ends here
