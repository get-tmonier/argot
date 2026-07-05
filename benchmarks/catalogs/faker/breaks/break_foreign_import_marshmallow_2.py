"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: submodule import from marshmallow to serialize generated profiles
# hunk starts here
from marshmallow import Schema
from marshmallow.fields import Integer, String


class ProfileSchema(Schema):
    name = String(required=True)
    email = String()
    age = Integer()


def serialize_profiles(rows: list[dict]) -> list[dict]:
    schema = ProfileSchema()
    return [schema.dump(r) for r in rows]
# hunk ends here
