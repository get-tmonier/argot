# Break: PyJWT signs bearer tokens instead of scrapy's HttpAuthMiddleware
"""Break fixture — not for import."""

# hunk starts here
import jwt


def build_auth_header(user: str, secret: str) -> str:
    token = jwt.encode({"sub": user}, secret, algorithm="HS256")
    return f"Bearer {token}"


def verify_token(token: str, secret: str) -> dict:
    return jwt.decode(token, secret, algorithms=["HS256"])
# hunk ends here
