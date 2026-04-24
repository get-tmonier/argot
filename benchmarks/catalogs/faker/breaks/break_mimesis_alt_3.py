"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# hunk starts here
def generate_financial_profile() -> dict[str, object]:
    fake = Faker()
    return {
        "aba": fake.aba(),
        "bban": fake.bban(),
        "iban": fake.iban(),
        "swift": fake.swift(length=11),
        "swift8": fake.swift8(),
        "swift11": fake.swift11(),
        "cryptocurrency_code": fake.cryptocurrency_code(),
        "cryptocurrency_name": fake.cryptocurrency_name(),
        "pricetag": fake.pricetag(),
    }
# hunk ends here
