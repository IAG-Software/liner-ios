from pathlib import Path

# 1. rename island_pill -> dynamic_island copy in every locale
LOCALES = Path(r"D:\ios2\liner-music-ios\src\languages\locales")
for path in sorted(LOCALES.glob("*.json")):
    import json

    data = json.loads(path.read_text(encoding="utf-8"))
    mobile = data.setdefault("settings", {}).setdefault("mobile", {})
    mobile.pop("island_pill", None)
    mobile["dynamic_island"] = {
        "title": "Dynamic Island & lock screen",
        "description": "Show what is playing in the Dynamic Island and on the lock screen. Turn off to keep Liner out of the island.",
    }
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

# 2. MobileTab: use the new key and clear the session when disabling
p = Path(r"D:\ios2\liner-music-ios\src\features\settings\ui\tabs\MobileTab.tsx")
s = p.read_text(encoding="utf-8")
s = s.replace('key: "islandPill" | "autoHideChrome" | "extraEdgeInset"',
              'key: "dynamicIsland" | "autoHideChrome" | "extraEdgeInset"')
s = s.replace('''          "islandPill",
          "settings.mobile.island_pill.title",
          "settings.mobile.island_pill.description",''',
              '''          "dynamicIsland",
          "settings.mobile.dynamic_island.title",
          "settings.mobile.dynamic_island.description",''')
s = s.replace("onClick={() => settings.set(key, !settings[key])}",
              "onClick={() => {\n"
              "            const next = !settings[key];\n"
              "            settings.set(key, next);\n"
              "            // Toggling this off must clear the published metadata now,\n"
              "            // otherwise iOS keeps showing the current track in the island.\n"
              "            if (key === \"dynamicIsland\" && !next && typeof navigator !== \"undefined\" && \"mediaSession\" in navigator) {\n"
              "              try {\n"
              "                navigator.mediaSession.metadata = null;\n"
              "                navigator.mediaSession.playbackState = \"none\";\n"
              "              } catch {\n"
              "                /* ignore */\n"
              "              }\n"
              "            }\n"
              "          }}")
p.write_text(s, encoding="utf-8")
print("locals + MobileTab patched")
