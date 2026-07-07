# ID: scrapy/responsetypes.py:106

def from_body(self, body):
    """Guess the Response class by sniffing the first bytes of the body."""
    sample = body[:5000]
    sample = to_bytes(sample)
    if not binary_is_text(sample):
        return self.from_mimetype("application/octet-stream")
    lowered = sample.lower()
    if b"<html>" in lowered:
        return self.from_mimetype("text/html")
    if b"<?xml" in lowered:
        return self.from_mimetype("text/xml")
    if b"<!doctype html>" in lowered:
        return self.from_mimetype("text/html")
    return self.from_mimetype("text")
