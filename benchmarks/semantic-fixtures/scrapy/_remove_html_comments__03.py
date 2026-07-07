# ID: scrapy/utils/response.py:63

def _strip_html_comments(body):
    cursor = body.find(b"<!--")
    while cursor != -1:
        closing = body.find(b"-->", cursor + 1)
        if closing == -1:
            return body[:cursor]
        body = body[:cursor] + body[closing + 3:]
        cursor = body.find(b"<!--")
    return body
