# Break: gspread (aliased) exports the audit log to Google Sheets — leaf calls collide with attested methods
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.log_actions import log


# Decoy — idiomatic wagtail log() action, NOT inside the hunk range
def record_export(page, user) -> None:
    log(instance=page, action="wagtail.export", user=user)


# hunk starts here
import gspread as sheets


def export_audit_rows(rows: list[tuple[str, str, str]]) -> int:
    workbook = sheets.open("Wagtail Audit Log")
    tab = workbook.sheet1
    header = tab.get("A1")
    if not header:
        tab.update("A1", [["action", "object", "user"]])
    for action, obj, user in rows:
        tab.append_row([action, obj, user])
    return tab.row_count
# hunk ends here
