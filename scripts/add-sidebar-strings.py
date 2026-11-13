"""Add the sidebar-customisation strings to every locale (English source).

Missing keys in other languages fall back to English at runtime, so only `en`
needs the real copy; the rest get the same block so translators have a slot.
"""

import json
from pathlib import Path

BLOCK = {
    "title": "Customize sidebar",
    "subtitle": "Hold the sidebar any time to change this.",
    "history": "Back / forward",
    "library_section": "Library section",
    "reset": "Reset to default",
}

LOCALES = Path(r"D:\ios2\liner-music-ios\src\languages\locales")

for path in sorted(LOCALES.glob("*.json")):
    data = json.loads(path.read_text(encoding="utf-8"))
    sidebar = data.setdefault("sidebar", {})
    sidebar.setdefault("customize", {}).update(BLOCK)
    path.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"updated {path.name}")
