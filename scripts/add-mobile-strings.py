"""Add the Mobile settings tab strings (English) to every locale."""

import json
from pathlib import Path

MOBILE = {
    "description": "Phone-only behaviour: idle chrome, the island pill and edge padding.",
    "section_player": "Player",
    "section_layout": "Layout",
    "island_pill": {
        "title": "Island pill",
        "description": "Show the provider/lyrics pill at the top of the fullscreen player.",
    },
    "auto_hide": {
        "title": "Hide controls when idle",
        "description": "Collapse the transport after 3 seconds without a touch, leaving only the lyrics. Tap to bring it back.",
    },
    "edge_inset": {
        "title": "Extra edge padding",
        "description": "Wider gutter so the sidebar clears the rounded display corners on newer iPhones.",
    },
}

LOCALES = Path(r"D:\ios2\liner-music-ios\src\languages\locales")

for path in sorted(LOCALES.glob("*.json")):
    data = json.loads(path.read_text(encoding="utf-8"))
    settings = data.setdefault("settings", {})
    tabs = settings.setdefault("tabs", {})
    tabs.setdefault("mobile", "Mobile")
    settings.setdefault("mobile", {}).update(MOBILE)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"updated {path.name}")
