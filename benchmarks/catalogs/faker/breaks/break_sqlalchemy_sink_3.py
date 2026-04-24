"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
from sqlalchemy.orm import Session


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# hunk starts here
class FakerUser:
    def __init__(self, name: str, email: str, city: str) -> None:
        self.name = name
        self.email = email
        self.city = city


def bulk_insert_fakes(session: Session, count: int = 100) -> None:
    fake = Faker()
    users = [
        FakerUser(name=fake.name(), email=fake.email(), city=fake.city())
        for _ in range(count)
    ]
    session.bulk_save_objects(users)
    session.flush()
    session.expire_all()
# hunk ends here
