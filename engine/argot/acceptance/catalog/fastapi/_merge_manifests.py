"""Throwaway script: merge all manifest_v3_*.json into manifest.json."""
from __future__ import annotations

import json
from pathlib import Path

CATALOG = Path(__file__).parent
FIXTURES_DIR = CATALOG / "fixtures" / "default"
MAIN_MANIFEST = CATALOG / "manifest.json"

# Standard FixtureSpec fields
REQUIRED = {"name", "scope", "file", "hunk_start_line", "hunk_end_line", "is_break", "category", "set", "rationale"}


def normalize(entry: dict, source_file: str) -> dict | None:  # type: ignore[type-arg]
    """Coerce non-standard entry to standard schema. Returns None if unfixable."""
    # Downstream-http style: id, label, hunk_lines
    if "id" in entry and "name" not in entry:
        entry["name"] = entry.pop("id")
    if "label" in entry:
        label = entry.pop("label")
        entry["is_break"] = label == "break"
    if "hunk_lines" in entry:
        hl = entry.pop("hunk_lines")
        entry["hunk_start_line"] = hl[0]
        entry["hunk_end_line"] = hl[1]
    if "hunk" in entry and isinstance(entry["hunk"], dict):
        h = entry.pop("hunk")
        entry["hunk_start_line"] = h["start_line"]
        entry["hunk_end_line"] = h["end_line"]
    # Dependency-injection style: verdict instead of is_break
    if "verdict" in entry:
        verdict = entry.pop("verdict")
        entry["is_break"] = verdict == "break"
    # Remove non-standard extra fields
    for key in list(entry.keys()):
        if key not in REQUIRED:
            entry.pop(key)
    # Fill missing required fields with defaults
    entry.setdefault("scope", "default")
    entry.setdefault("set", "v3")
    # For entries missing hunk ranges, find first/last endpoint in the file
    if "hunk_start_line" not in entry or "hunk_end_line" not in entry:
        py_file = CATALOG / entry["file"]
        if py_file.exists():
            import ast as _ast
            tree = _ast.parse(py_file.read_text())
            funcs = [n for n in _ast.walk(tree)
                     if isinstance(n, (_ast.FunctionDef, _ast.AsyncFunctionDef))]
            if funcs:
                # Use the range from first decorated endpoint to last line
                decorated = [f for f in funcs if f.decorator_list]
                if decorated:
                    entry["hunk_start_line"] = decorated[0].lineno
                    entry["hunk_end_line"] = max(f.end_lineno for f in decorated)  # type: ignore[attr-defined]
                else:
                    entry["hunk_start_line"] = funcs[0].lineno
                    entry["hunk_end_line"] = funcs[-1].end_lineno  # type: ignore[attr-defined]
            else:
                print(f"  WARN: no functions in {py_file}, skipping")
                return None
        else:
            print(f"  WARN: file not found {py_file}, skipping")
            return None
    if "rationale" not in entry:
        entry["rationale"] = entry.pop("description", "")
    # Validate file exists
    py_file = CATALOG / entry["file"]
    if not py_file.exists():
        print(f"  SKIP: fixture file missing: {py_file}")
        return None
    # Validate hunk range
    line_count = len(py_file.read_text().splitlines())
    if entry["hunk_end_line"] > line_count:
        print(f"  WARN: {entry['name']} hunk_end_line {entry['hunk_end_line']} > file length {line_count}, clamping")
        entry["hunk_end_line"] = line_count
    return entry


def main() -> None:
    # Load current manifest
    data = json.loads(MAIN_MANIFEST.read_text())
    existing: dict[str, dict] = {f["name"]: f for f in data["fixtures"]}  # type: ignore[type-arg]
    removes: set[str] = set()

    # Collect removes
    for rm_file in sorted(CATALOG.glob("manifest_v3_*_removes.json")):
        rm_data = json.loads(rm_file.read_text())
        removes.update(rm_data.get("remove", []))
    print(f"Removing: {sorted(removes)}")

    # Collect new entries from partial manifests
    new_entries: list[dict] = []  # type: ignore[type-arg]
    for partial in sorted(CATALOG.glob("manifest_v3_*.json")):
        if "_removes" in partial.name:
            continue
        pdata = json.loads(partial.read_text())
        fixtures = pdata.get("fixtures", [])
        print(f"\nProcessing {partial.name} ({len(fixtures)} entries):")
        for entry in fixtures:
            entry = dict(entry)
            normalized = normalize(entry, partial.name)
            if normalized is None:
                continue
            name = normalized["name"]
            if name in existing:
                print(f"  SKIP: {name} already in manifest")
                continue
            print(f"  ADD: {name} (is_break={normalized['is_break']}, lines {normalized['hunk_start_line']}-{normalized['hunk_end_line']})")
            new_entries.append(normalized)

    # Build merged fixture list
    merged = [f for f in data["fixtures"] if f["name"] not in removes]
    print(f"\nRemoved {len(data['fixtures']) - len(merged)} fixtures")
    merged.extend(new_entries)
    print(f"Added {len(new_entries)} new fixtures")
    print(f"Total: {len(merged)} fixtures")

    # Write merged manifest
    data["fixtures"] = merged
    MAIN_MANIFEST.write_text(json.dumps(data, indent=2) + "\n")
    print(f"\nWrote {MAIN_MANIFEST}")


if __name__ == "__main__":
    main()
