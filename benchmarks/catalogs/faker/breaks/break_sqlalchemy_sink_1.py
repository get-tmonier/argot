"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def fake_company_data() -> dict[str, str]:
    fake = Faker()
    return {
        "name": fake.company(),
        "bs": fake.bs(),
        "catch_phrase": fake.catch_phrase(),
    }


# hunk starts here
from sqlalchemy import Column, Integer, String, create_engine
from sqlalchemy.orm import DeclarativeBase, Session


class Base(DeclarativeBase):
    pass


class UserRecord(Base):
    __tablename__ = "users"
    id = Column(Integer, primary_key=True, autoincrement=True)
    name = Column(String(120), nullable=False)
    email = Column(String(200), nullable=False)
    city = Column(String(100))


def seed_fake_users(session: Session, count: int = 20) -> None:
    fake = Faker()
    for _ in range(count):
        user = UserRecord(
            name=fake.name(),
            email=fake.email(),
            city=fake.city(),
        )
        session.add(user)
    session.commit()


def build_engine(url: str = "sqlite:///:memory:") -> object:
    engine = create_engine(url, echo=False)
    Base.metadata.create_all(engine)
    return engine
# hunk ends here
