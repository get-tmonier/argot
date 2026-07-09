"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: import orjson and serialize generated ISBN records with it instead of stdlib json
# hunk starts here
import orjson


def serialize_isbn_records(records: list[dict]) -> bytes:
    payload = orjson.dumps(records, option=orjson.OPT_SORT_KEYS)
    return payload
# hunk ends here
