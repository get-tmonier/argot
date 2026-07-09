"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
from loguru import logger


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: loguru logger import sits in the decoy region; hunk binds and logs generated passports
# hunk starts here
def log_generated_passports(passports: list[str]) -> None:
    bound = logger.bind(source="passport_provider")
    for number in passports:
        bound.info("generated passport {}", number)
# hunk ends here
