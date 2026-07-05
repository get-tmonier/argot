"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: elasticsearch client on self; leaf methods collide with attested get/update
# hunk starts here
class PlateIndexer(BaseProvider):
    def index_plates(self, plates: list[str]) -> dict:
        for i, plate in enumerate(plates):
            self._es.update(index="plates", id=str(i), body={"doc": {"plate": plate}})
        first = self._es.get(index="plates", id="0")
        return first.get("_source", {})
# hunk ends here
