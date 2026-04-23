"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {
        "name": fake.name(),
        "email": fake.email(),
        "address": fake.address(),
        "phone": fake.phone_number(),
        "company": fake.company(),
    }


# hunk starts here
from mimesis import Person, Address, Finance
from mimesis.enums import Gender

_person = Person("en")
_address = Address("en")
_finance = Finance("en")


def generate_mimesis_person(gender: str = "male") -> dict[str, str]:
    g = Gender.MALE if gender == "male" else Gender.FEMALE
    return {
        "full_name": _person.full_name(gender=g),
        "email": _person.email(),
        "phone": _person.telephone(),
        "username": _person.username(),
        "age": str(_person.age(minimum=18, maximum=65)),
    }


def generate_mimesis_address() -> dict[str, str]:
    return {
        "city": _address.city(),
        "country": _address.country(),
        "street": _address.street_name(),
        "zip": _address.postal_code(),
        "calling_code": _address.calling_code(),
    }
# hunk ends here
