import json
from pathlib import Path

DESC = (
    "Phone-only behaviour: Dynamic Island and lock-screen presence, idle chrome, "
    "and the gutter that keeps the sidebar off the rounded corners."
)

for path in sorted(Path(r"D:\ios2\liner-music-ios\src\languages\locales").glob("*.json")):
    data = json.loads(path.read_text(encoding="utf-8"))
    data.setdefault("settings", {}).setdefault("mobile", {})["description"] = DESC
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print("description updated")
