from datetime import date


def render_day() -> str:
    return date.today().isoformat()
