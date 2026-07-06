# Break: pydantic BaseModel validates config instead of scrapy's Settings
"""Break fixture — not for import."""

# hunk starts here
import pydantic


class DownloaderConfig(pydantic.BaseModel):
    concurrent_requests: int = pydantic.Field(16, ge=1)
    download_delay: float = 0.0


def load_config(raw: dict) -> DownloaderConfig:
    return DownloaderConfig.model_validate(raw)
# hunk ends here
