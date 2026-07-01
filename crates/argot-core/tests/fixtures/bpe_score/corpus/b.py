import json
from pathlib import Path


class Config:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.data = json.loads(path.read_text())

    def get(self, key, default=None):
        return self.data.get(key, default)
