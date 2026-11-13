from pathlib import Path

p = Path(r"D:\ios2\liner-music-ios\scripts\ios-build.sh")
s = p.read_text(encoding="utf-8")

# the source line landed *after* the call site, so the function did not exist yet
s = s.replace(
    '# App extensions (Today widget, iMessage, Intents) live in PlugIns/.\nsource "$SCRIPT_DIR/build-extensions.sh"\n\n',
    "",
)
if 'source "$SCRIPT_DIR/build-extensions.sh"' not in s:
    s = s.replace(
        'mkdir -p "$BUILD_DIR"\n',
        'mkdir -p "$BUILD_DIR"\n\n'
        '# App extensions (Today widget, iMessage, Intents) live in PlugIns/.\n'
        'source "$SCRIPT_DIR/build-extensions.sh"\n',
    )
p.write_text(s, encoding="utf-8")
print("source moved")
