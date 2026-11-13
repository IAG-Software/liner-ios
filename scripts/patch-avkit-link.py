from pathlib import Path

# AVKit is needed for AVRoutePickerView
p = Path(r"D:\ios2\liner-music-ios\scripts\ios-build.sh")
s = p.read_text(encoding="utf-8")
if "-framework AVKit" not in s:
    s = s.replace("  -framework AVFoundation \\", "  -framework AVFoundation \\\n  -framework AVKit \\")
p.write_text(s, encoding="utf-8")

p = Path(r"D:\ios2\liner-music-ios\src-tauri\src\platform_ios.rs")
s = p.read_text(encoding="utf-8")
if 'name = "AVKit"' not in s:
    s = s.replace(
        '#[link(name = "WebKit", kind = "framework")]',
        '#[link(name = "WebKit", kind = "framework")]\n#[link(name = "AVKit", kind = "framework")]',
    )
p.write_text(s, encoding="utf-8")
print("AVKit linked")
