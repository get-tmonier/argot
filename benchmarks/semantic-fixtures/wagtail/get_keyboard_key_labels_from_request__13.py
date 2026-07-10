# ID: wagtail/admin/utils.py:87
def keyboard_labels_for_request(request):
    """Return a SimpleNamespace of keyboard key labels, using macOS glyphs when the User-Agent looks like Apple hardware."""
    user_agent = request.headers.get("User-Agent", "")
    is_mac_os = re.search(r"Mac|iPod|iPhone|iPad", user_agent)

    labels = {
        "ALT": "⌥" if is_mac_os else "Alt",
        "CMD": "⌘" if is_mac_os else "Ctrl",
        "CTRL": "^" if is_mac_os else "Ctrl",
        "DEL": "Delete",
        "ENTER": "Return" if is_mac_os else "Enter",
        "ESC": "Esc",
        "MOD": "⌘" if is_mac_os else "Ctrl",
        "SHIFT": "Shift",
        "TAB": "Tab",
    }

    return SimpleNamespace(**labels)
