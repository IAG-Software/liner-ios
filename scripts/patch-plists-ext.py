from pathlib import Path

base = Path(r"D:\ios2\liner-music-ios\ios")

# --- add CFBundleExecutable to each extension plist -------------------------
for folder, exe in [
    ("LinerWidget", "LinerWidget"),
    ("LinerMessages", "LinerMessages"),
    ("LinerIntents", "LinerIntents"),
]:
    p = base / folder / "Info.plist"
    s = p.read_text(encoding="utf-8")
    if "CFBundleExecutable" not in s:
        s = s.replace(
            "\t<key>CFBundleIdentifier</key>",
            f"\t<key>CFBundleExecutable</key>\n\t<string>{exe}</string>\n\t<key>CFBundleIdentifier</key>",
        )
        p.write_text(s, encoding="utf-8")

# --- URL scheme on the app -------------------------------------------------
p = base / "LinerMusic" / "Info.plist"
s = p.read_text(encoding="utf-8")
if "CFBundleURLTypes" not in s:
    s = s.replace(
        "\t<key>CFBundleVersion</key>",
        "\t<key>CFBundleVersion</key>",
    )
    s = s.replace(
        "\t<key>LSRequiresIPhoneOS</key>",
        """\t<key>CFBundleURLTypes</key>
\t<array>
\t\t<dict>
\t\t\t<key>CFBundleURLName</key>
\t\t\t<string>com.iagsoftware.linermusic</string>
\t\t\t<key>CFBundleURLSchemes</key>
\t\t\t<array>
\t\t\t\t<string>liner</string>
\t\t\t</array>
\t\t</dict>
\t</array>
\t<key>LSRequiresIPhoneOS</key>""",
    )
    p.write_text(s, encoding="utf-8")

print("plists updated")
