# ID: rich/_emoji_replace.py:9
def _replace_emoji_codes(
    text: str,
    default_variant: Optional[str] = None,
    _emoji_sub: _EmojiSubMethod = re.compile(r"(:(\S*?)(?:(?:\-)(emoji|text))?:)").sub,
) -> str:
    """Replace emoji shortcodes in text with their unicode characters."""
    from ._emoji_codes import EMOJI

    lookup_emoji = EMOJI.__getitem__
    variant_codes = {"text": "︎", "emoji": "️"}
    lookup_variant = variant_codes.get
    fallback_variant = (
        variant_codes.get(default_variant, "") if default_variant else ""
    )

    def substitute(match: Match[str]) -> str:
        raw_code, emoji_name, variant = match.groups()
        try:
            return lookup_emoji(emoji_name.lower()) + lookup_variant(
                variant, fallback_variant
            )
        except KeyError:
            return raw_code

    return _emoji_sub(substitute, text)
