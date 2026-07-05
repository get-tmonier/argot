"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: pymongo collection reached through an injected receiver — no import in the diff
# hunk starts here
def store_generated_places(collection, places: list[dict]) -> int:
    for place in places:
        collection.insert_one(place)
    collection.update_many({}, {"$set": {"synthetic": True}})
    return collection.count_documents({})
# hunk ends here
