# Break fixture -- not for compilation into the build.
from rich.console import Console

# Decoy: idiomatic rich-style helper.
def existing_repr_helper(obj):
    return getattr(obj, "__rich_repr__", None)

# Break: pydantic BaseModel validates and serializes object fields instead of rich's own repr protocol.
# Break: Zero `pydantic` sites in the repo at the pinned SHA (git grep -w pydantic over the repo = 0; not a declared dependency).
from pydantic import BaseModel

class ReprFields(BaseModel):
    name: str
    value: str

    def to_json(self):
        return self.model_dump_json()
