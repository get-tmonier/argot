"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
from polyfactory.factories import ModelFactory
from pydantic import BaseModel


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# hunk starts here
class UserModel(BaseModel):
    name: str
    email: str
    age: int
    city: str


class UserModelFactory(ModelFactory):
    __model__ = UserModel

    @classmethod
    def build_batch_with_locale(cls, size: int) -> list[UserModel]:
        return [cls.build() for _ in range(size)]


def generate_users_polyfactory(count: int = 10) -> list[dict[str, object]]:
    users = UserModelFactory.build_batch_with_locale(count)
    return [u.model_dump() for u in users]
# hunk ends here
