"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: stripe payment API — fully-qualified foreign calls to mint fake charges
# hunk starts here
import stripe


def create_fake_charge(amount_cents: int = 500) -> str:
    stripe.api_key = "sk_test_fake"
    customer = stripe.Customer.create(name=Faker().name(), email=Faker().email())
    charge = stripe.Charge.create(amount=amount_cents, currency="usd", customer=customer.id)
    return charge.id
# hunk ends here
